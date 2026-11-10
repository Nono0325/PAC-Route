pub mod dueling_ddqn;
pub mod latency_model;

pub use dueling_ddqn::{ActionEvaluation, DuelingDdqnScheduler};
pub use latency_model::LatencyPredictor;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pac_core::types::{
        ChatMessage, NodeHardware, NodeInfo, NodeRole, NodeStatus, PacInferenceRequest,
        PrivacyLevel, RoutingMode, SlaPolicy,
    };

    fn make_test_node(id: &str, role: NodeRole, token_speed: f64, queue_len: usize) -> NodeInfo {
        NodeInfo {
            id: id.to_string(),
            name: format!("Node {}", id),
            role,
            endpoint_url: "http://127.0.0.1:11434".to_string(),
            hardware: NodeHardware {
                gpu_model: Some("Test GPU".to_string()),
                total_vram_mb: 16000,
                free_vram_mb: 8000,
                cpu_cores: 8,
                total_ram_mb: 32000,
                free_ram_mb: 16000,
                token_speed_estimate: token_speed,
            },
            supported_models: vec!["llama3.2:3b".to_string()],
            loaded_models: vec!["llama3.2:3b".to_string()],
            current_queue_len: queue_len,
            network_rtt_ms: 2.0,
            status: NodeStatus::Online,
            last_heartbeat: Utc::now(),
        }
    }

    #[test]
    fn test_latency_prediction() {
        let req = PacInferenceRequest::new(
            "llama3.2:3b",
            vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello world test prompt".to_string(),
            }],
        );
        let node = make_test_node("gpu_fast", NodeRole::WorkstationGpu, 70.0, 0);
        let latency = LatencyPredictor::predict(&req, &node);

        assert!(latency.exec_ms > 0.0);
        assert_eq!(latency.load_ms, 0.0); // 因為模型已載入
        assert!(latency.total_ms > 0.0);
    }

    #[test]
    fn test_sla_action_masking_blocks_overdue_node() {
        let scheduler = DuelingDdqnScheduler::new();
        let mut req = PacInferenceRequest::new(
            "llama3.2:3b",
            vec![ChatMessage {
                role: "user".to_string(),
                content: "Fast request".to_string(),
            }],
        );
        // 設定極嚴苛之 SLA Deadline：300ms
        req.sla = SlaPolicy {
            deadline_ms: 300.0,
            strict_action_masking: true,
            max_queue_wait_ms: 500.0,
        };

        // 節點 1: 佇列很長，預估耗時必然大於 300ms
        let slow_busy_node = make_test_node("slow_node", NodeRole::EdgeGpu, 20.0, 5);

        let candidates = vec![slow_busy_node];
        let result = scheduler.schedule_request(
            &req,
            PrivacyLevel::Public,
            RoutingMode::DirectLocal,
            &candidates,
        );

        // 必須觸發 SLA 違約錯誤，動作被 Mask 掉
        assert!(result.is_err());
    }

    #[test]
    fn test_top_secret_blocks_cloud_proxy() {
        let scheduler = DuelingDdqnScheduler::new();
        let req = PacInferenceRequest::new(
            "llama3.2:3b",
            vec![ChatMessage {
                role: "user".to_string(),
                content: "機密財務報表與個人密碼".to_string(),
            }],
        );

        // 只有雲端節點
        let cloud_node = make_test_node("cloud_proxy", NodeRole::CloudProxy, 100.0, 0);
        let candidates = vec![cloud_node];

        let result = scheduler.schedule_request(
            &req,
            PrivacyLevel::TopSecret,
            RoutingMode::DirectLocal,
            &candidates,
        );

        // 機密資料禁止走雲端，動作必須被 Mask
        assert!(result.is_err());
    }
}
