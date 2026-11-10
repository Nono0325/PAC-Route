use std::time::Duration;
use chrono::Utc;
use tracing::{error, info};
use pac_core::types::{NodeHardware, NodeInfo, NodeRole, NodeStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let node_id = std::env::var("PAC_NODE_ID").unwrap_or_else(|_| "node_rpi5_custom".to_string());
    let router_url = std::env::var("PAC_ROUTER_URL").unwrap_or_else(|_| "http://127.0.0.1:8088".to_string());

    info!("Starting PAC Node Agent for node '{}'", node_id);
    info!("Target PAC-Route: {}", router_url);

    let client = reqwest::Client::new();

    // 模擬週期性採集本機硬體負載與顯存狀態
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let heartbeat_payload = NodeInfo {
            id: node_id.clone(),
            name: format!("Edge Agent - {}", node_id),
            role: NodeRole::EdgeSbc,
            endpoint_url: "http://127.0.0.1:11434".to_string(),
            hardware: NodeHardware {
                gpu_model: None,
                total_vram_mb: 0,
                free_vram_mb: 0,
                cpu_cores: 4,
                total_ram_mb: 8192,
                free_ram_mb: 4200,
                token_speed_estimate: 9.5,
            },
            supported_models: vec!["llama3.2:1b".to_string()],
            loaded_models: vec!["llama3.2:1b".to_string()],
            current_queue_len: 0,
            network_rtt_ms: 3.5,
            status: NodeStatus::Online,
            last_heartbeat: Utc::now(),
        };

        // 發送心跳或註冊資訊
        let res = client
            .post(format!("{}/health", router_url))
            .json(&heartbeat_payload)
            .send()
            .await;

        match res {
            Ok(resp) => {
                info!("Node heartbeat sent, router responded with status: {}", resp.status());
            }
            Err(e) => {
                error!("Failed to reach PAC-Route: {}", e);
            }
        }
    }
}

