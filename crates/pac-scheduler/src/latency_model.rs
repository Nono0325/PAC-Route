use pac_core::types::{LatencyBreakdown, NodeInfo, PacInferenceRequest};

/// 延遲評估模型 (Formal Latency Predictor)
/// 實作論文公式：T_total = T_queue + T_load + T_trans + T_exec
pub struct LatencyPredictor;

impl LatencyPredictor {
    /// 預測請求分配至特定節點的端到端耗時與各階段拆解
    pub fn predict(request: &PacInferenceRequest, node: &NodeInfo) -> LatencyBreakdown {
        let prompt_tokens = request.estimate_token_count();
        let max_output_tokens = request.max_tokens.unwrap_or(256).min(512);

        // 1. 估算傳輸延遲 T_trans: (Prompt 長度 / 估計頻寬) + 往返時延 RTT
        let prompt_bytes = request.prompt_text().len() as f64;
        let bandwidth_kb_per_ms = match node.role {
            pac_core::types::NodeRole::CloudProxy => 5.0,     // 外部網路約 50Mbps
            pac_core::types::NodeRole::EdgeSbc => 10.0,       // 局域網 WiFi
            _ => 100.0,                                      // 局域網 Gigabit LAN
        };
        let trans_ms = (prompt_bytes / (bandwidth_kb_per_ms * 1024.0)) + node.network_rtt_ms;

        // 2. 估算模型載入/顯存換入開銷 T_load
        let is_loaded = node.loaded_models.iter().any(|m| m == &request.model);
        let load_ms = if is_loaded {
            0.0
        } else {
            // 若未載入，高階 GPU 需約 800ms，SBC 需 3000ms+
            match node.role {
                pac_core::types::NodeRole::WorkstationGpu => 600.0,
                pac_core::types::NodeRole::EdgeGpu => 1200.0,
                pac_core::types::NodeRole::EdgeSbc => 3500.0,
                pac_core::types::NodeRole::CloudProxy => 0.0, // 雲端通常熱載入或自動擴容
                _ => 1000.0,
            }
        };

        // 3. 估算實際推論執行時間 T_exec:
        // Prefill 時間 (每千字約數毫秒) + 解碼時間 (Token / token_speed_estimate)
        let token_speed = if node.hardware.token_speed_estimate > 0.0 {
            node.hardware.token_speed_estimate
        } else {
            10.0
        };
        let prefill_ms = (prompt_tokens as f64) * 0.15;
        let decode_ms = (max_output_tokens as f64 / token_speed) * 1000.0;
        let exec_ms = prefill_ms + decode_ms;

        // 4. 估算佇列等待時間 T_queue
        // 依據目前佇列深度與平均單任務時間推估
        let queue_ms = (node.current_queue_len as f64) * (exec_ms * 0.6);

        let total_ms = queue_ms + trans_ms + load_ms + exec_ms;

        LatencyBreakdown {
            queue_ms,
            trans_ms,
            load_ms,
            exec_ms,
            total_ms,
        }
    }
}
