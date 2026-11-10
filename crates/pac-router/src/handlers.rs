use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use pac_core::types::{
    ChatMessage, PacInferenceRequest, PrivacyLevel, RoutingMode, SlaPolicy,
};
use pac_privacy::SoftGatingClassifier;
use pac_scheduler::LatencyPredictor;
use crate::state::AppState;

/// OpenAI 格式 Chat Completion 請求體
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub stream: Option<bool>,
}

/// 健康檢查
pub async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let nodes = state.registry.list_active_nodes().await;
    Json(json!({
        "status": "healthy",
        "service": "PAC-Route",
        "version": pac_core::PAC_ROUTE_VERSION,
        "active_nodes_count": nodes.len(),
        "timestamp": Utc::now()
    }))
}

/// 查詢叢集當前所有節點狀態
pub async fn cluster_nodes_handler(State(state): State<AppState>) -> impl IntoResponse {
    let nodes = state.registry.list_active_nodes().await;
    Json(json!({
        "nodes": nodes
    }))
}

/// OpenAI 相容 /v1/models 模型清單
pub async fn list_models_handler(State(state): State<AppState>) -> impl IntoResponse {
    let nodes = state.registry.list_active_nodes().await;
    let mut model_set = std::collections::HashSet::new();
    for node in nodes {
        for m in node.supported_models {
            model_set.insert(m);
        }
    }

    let models_data: Vec<_> = model_set
        .into_iter()
        .map(|m| {
            json!({
                "id": m,
                "object": "model",
                "owned_by": "pac-route-cluster",
                "permission": []
            })
        })
        .collect();

    Json(json!({
        "object": "list",
        "data": models_data
    }))
}

/// 核心：OpenAI 相容 /v1/chat/completions 路由端點
pub async fn chat_completions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OpenAiChatRequest>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let start_instant = std::time::Instant::now();

    // 1. 解析 SLA 與隱私自訂 Header
    let deadline_ms = headers
        .get("x-sla-deadline-ms")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(3000.0);

    let explicit_privacy = headers
        .get("x-privacy-level")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s.to_lowercase().as_str() {
            "topsecret" | "secret" => Some(PrivacyLevel::TopSecret),
            "restricted" => Some(PrivacyLevel::Restricted),
            "public" => Some(PrivacyLevel::Public),
            _ => None,
        });

    let consent_cloud = headers
        .get("x-consent-cloud")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == "true" || s == "1")
        .unwrap_or(false);

    let mut pac_req = PacInferenceRequest::new(payload.model.clone(), payload.messages);
    pac_req.max_tokens = payload.max_tokens;
    pac_req.temperature = payload.temperature;
    pac_req.stream = payload.stream.unwrap_or(false);
    pac_req.sla = SlaPolicy {
        deadline_ms,
        strict_action_masking: true,
        max_queue_wait_ms: 1500.0,
    };
    pac_req.explicit_privacy_level = explicit_privacy;
    pac_req.user_consent_for_cloud = consent_cloud;

    // 2. 軟體閘門 (Soft Gating)：分類隱私等級與執行模式
    let (detected_privacy, routing_mode) = SoftGatingClassifier::classify(&pac_req);

    // 3. 局域網能力發現：取得合適候選節點
    let candidates = state.registry.get_candidates_for_model(&pac_req.model).await;
    if candidates.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "message": format!("在當前個人叢集中未發現支援模型 '{}' 之可用節點", pac_req.model),
                    "type": "no_candidate_node",
                    "code": 404
                }
            })),
        ));
    }

    // 4. 智慧調度 (Dueling DDQN + SLA Action-Masking)
    let (chosen_node, action_eval) = match state.scheduler.schedule_request(
        &pac_req,
        detected_privacy,
        routing_mode,
        &candidates,
    ) {
        Ok(res) => res,
        Err(e) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": {
                        "message": format!("PAC-Route SLA 調度失敗：{}", e),
                        "type": "sla_violation_or_mask",
                        "code": 503
                    }
                })),
            ));
        }
    };

    info!(
        "PAC-Route 成功排程: 請求 {} -> 節點 {} ({:?}), 模式: {:?}, 隱私: {:?}",
        pac_req.id, chosen_node.name, chosen_node.role, routing_mode, detected_privacy
    );

    // 5. 執行推論與上下文協同
    let (final_content, latency_info) = match routing_mode {
        RoutingMode::SemanticSketchCollaborative => {
            // 雲邊協同語義草圖管線
            let prompt_text = pac_req.prompt_text();
            let (_sketch_prompt, entity_map) = state.sketch_pipeline.prepare_sketch_prompt(&prompt_text);

            // 模擬向雲端發送脫敏草圖並獲取結構
            let cloud_draft = format!(
                "【雲端抽象語義草圖已生成】針對問題分析如下：\n- 系統已識別關鍵實體占位符。\n- 依據業務邏輯，將對 {} 進行結構化處置與回應。",
                entity_map.keys().cloned().collect::<Vec<_>>().join(", ")
            );

            // 本地端實體重構還原
            let reconstructed = state
                .sketch_pipeline
                .reconstruct_final_output(&cloud_draft, &entity_map);

            let latency = LatencyPredictor::predict(&pac_req, &chosen_node);
            (reconstructed, latency)
        }
        RoutingMode::DirectLocal | RoutingMode::DirectCloud => {
            // 本地或直連雲端推論
            let simulated_content = format!(
                "【PAC-Route 叢集推論回應】\n由節點 [{}] 執行完成。\n模式：{:?}\n您的輸入已安全處理：{}",
                chosen_node.name,
                routing_mode,
                pac_req.prompt_text()
            );

            let latency = LatencyPredictor::predict(&pac_req, &chosen_node);
            (simulated_content, latency)
        }
    };

    let total_elapsed = start_instant.elapsed().as_secs_f64() * 1000.0;
    let prompt_tokens = pac_req.estimate_token_count();
    let completion_tokens = (final_content.chars().count() / 2).max(1);

    // 6. 回傳標準 OpenAI Chat Completion JSON
    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let response_body = json!({
        "id": completion_id,
        "object": "chat.completion",
        "created": Utc::now().timestamp(),
        "model": pac_req.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": final_content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens
        },
        "pac_meta": {
            "node_id": chosen_node.id,
            "node_name": chosen_node.name,
            "routing_mode": routing_mode,
            "privacy_level": detected_privacy,
            "sla_deadline_ms": deadline_ms,
            "predicted_ms": action_eval.predicted_total_ms,
            "actual_ms": total_elapsed,
            "breakdown": latency_info
        }
    });

    let mut res = Json(response_body).into_response();
    let res_headers = res.headers_mut();
    res_headers.insert("x-pac-node-id", chosen_node.id.parse().unwrap());
    res_headers.insert("x-pac-routing-mode", format!("{:?}", routing_mode).parse().unwrap());
    res_headers.insert("x-pac-privacy-level", format!("{:?}", detected_privacy).parse().unwrap());

    Ok(res)
}
