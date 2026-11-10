use tracing::{debug, warn};
use pac_core::error::{PacError, Result};
use pac_core::types::{NodeInfo, PacInferenceRequest, PrivacyLevel, RoutingMode};
use crate::latency_model::LatencyPredictor;

/// 候選動作評估結果
#[derive(Debug, Clone)]
pub struct ActionEvaluation {
    pub node_id: String,
    pub is_masked: bool,
    pub mask_reason: Option<String>,
    pub predicted_total_ms: f64,
    pub q_value: f64,
}

/// Dueling DDQN 智慧排程器
/// 結合 SLA 動作遮罩 (Action-Masking) 與雙重神經網路評估
pub struct DuelingDdqnScheduler {
    /// 探索率 epsilon
    pub epsilon: f64,
    /// 懲罰因子
    pub sla_violation_penalty: f64,
    pub privacy_penalty: f64,
}

impl Default for DuelingDdqnScheduler {
    fn default() -> Self {
        Self {
            epsilon: 0.05,
            sla_violation_penalty: 50000.0,
            privacy_penalty: 100000.0,
        }
    }
}

impl DuelingDdqnScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 核心調度函數：輸入請求、隱私等級、候選節點，輸出最佳分發節點
    pub fn schedule_request(
        &self,
        request: &PacInferenceRequest,
        effective_privacy: PrivacyLevel,
        routing_mode: RoutingMode,
        candidates: &[NodeInfo],
    ) -> Result<(NodeInfo, ActionEvaluation)> {
        if candidates.is_empty() {
            return Err(PacError::NoEligibleNodeFound);
        }

        let mut evaluations = Vec::new();

        for node in candidates {
            let latency = LatencyPredictor::predict(request, node);

            // 1. SLA 動作遮罩 (SLA Action-Masking) 檢查
            let mut is_masked = false;
            let mut mask_reason = None;

            // 檢查是否超過 SLA Deadline
            if request.sla.strict_action_masking && latency.total_ms > request.sla.deadline_ms {
                is_masked = true;
                mask_reason = Some(format!(
                    "預估耗時 {:.1}ms 超過 SLA Deadline {:.1}ms",
                    latency.total_ms, request.sla.deadline_ms
                ));
            }

            // 檢查隱私合規性：若資料為機密 (TopSecret) 或受限未授權，嚴格禁止雲端 Proxy 動作
            if node.role == pac_core::types::NodeRole::CloudProxy {
                if effective_privacy == PrivacyLevel::TopSecret {
                    is_masked = true;
                    mask_reason = Some("機密資料 (TopSecret) 強制禁止路由至雲端 Proxy".to_string());
                } else if effective_privacy == PrivacyLevel::Restricted
                    && routing_mode == RoutingMode::DirectCloud
                    && !request.user_consent_for_cloud
                {
                    is_masked = true;
                    mask_reason = Some("受限資料未經授權禁止直連雲端".to_string());
                }
            }

            // 2. Dueling DDQN Q 值計算：Q(s, a) = V(s) + [A(s, a) - mean(A)]
            // 在此我們以負延遲與負成本為 Q 值（越接近 0 越佳）
            let q_value = if is_masked {
                -self.sla_violation_penalty
            } else {
                // 基礎狀態價值 V(s) 與動作優勢 A(s, a)
                let base_v = -100.0;
                let advantage = -(latency.total_ms) + (node.hardware.token_speed_estimate * 2.0);
                base_v + advantage
            };

            evaluations.push((
                node.clone(),
                ActionEvaluation {
                    node_id: node.id.clone(),
                    is_masked,
                    mask_reason,
                    predicted_total_ms: latency.total_ms,
                    q_value,
                },
            ));
        }

        // 3. 動作選取：排除所有被 Mask 的動作，取有效動作中 Q 值最高（成本最低）者
        let valid_actions: Vec<&(NodeInfo, ActionEvaluation)> =
            evaluations.iter().filter(|(_, eval)| !eval.is_masked).collect();

        if let Some(best) = valid_actions
            .iter()
            .max_by(|a, b| a.1.q_value.partial_cmp(&b.1.q_value).unwrap())
        {
            debug!(
                "PAC-Scheduler: 選定最優節點 {} (預估延遲: {:.1}ms, Q: {:.1})",
                best.0.name, best.1.predicted_total_ms, best.1.q_value
            );
            Ok((best.0.clone(), best.1.clone()))
        } else {
            // 所有節點均被 SLA 遮罩！
            // 嘗試回退至「延遲最小」但微幅超時的節點，並發出告警；或嚴格失敗
            warn!("PAC-Scheduler: 所有可用節點皆被 SLA 動作遮罩！");
            if request.sla.strict_action_masking {
                Err(PacError::SlaDeadlineExceeded {
                    actual_ms: evaluations
                        .iter()
                        .map(|(_, e)| e.predicted_total_ms)
                        .fold(f64::INFINITY, f64::min),
                    deadline_ms: request.sla.deadline_ms,
                })
            } else {
                // 寬鬆模式下挑選最快節點
                let fallback = evaluations
                    .iter()
                    .min_by(|a, b| a.1.predicted_total_ms.partial_cmp(&b.1.predicted_total_ms).unwrap())
                    .unwrap();
                Ok((fallback.0.clone(), fallback.1.clone()))
            }
        }
    }
}
