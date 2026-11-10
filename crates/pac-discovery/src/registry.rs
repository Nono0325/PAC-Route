use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Duration, Utc};
use pac_core::types::{NodeHardware, NodeInfo, NodeRole, NodeStatus};

/// 節點能力註冊表 (Thread-Safe Capability Registry)
#[derive(Clone, Default)]
pub struct NodeRegistry {
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 註冊或更新節點資訊
    pub async fn register_or_update(&self, node: NodeInfo) {
        let mut map = self.nodes.write().await;
        map.insert(node.id.clone(), node);
    }

    /// 回報節點心跳與負載
    pub async fn update_heartbeat(
        &self,
        node_id: &str,
        queue_len: usize,
        free_vram_mb: u64,
        free_ram_mb: u64,
    ) -> bool {
        let mut map = self.nodes.write().await;
        if let Some(node) = map.get_mut(node_id) {
            node.current_queue_len = queue_len;
            node.hardware.free_vram_mb = free_vram_mb;
            node.hardware.free_ram_mb = free_ram_mb;
            node.last_heartbeat = Utc::now();
            node.status = NodeStatus::Online;
            true
        } else {
            false
        }
    }

    /// 取得所有當前在線節點列表
    pub async fn list_active_nodes(&self) -> Vec<NodeInfo> {
        let map = self.nodes.read().await;
        map.values()
            .filter(|n| n.status == NodeStatus::Online || n.status == NodeStatus::Busy)
            .cloned()
            .collect()
    }

    /// 依據模型名稱取得合適的候選節點列表
    pub async fn get_candidates_for_model(&self, model: &str) -> Vec<NodeInfo> {
        let map = self.nodes.read().await;
        map.values()
            .filter(|n| {
                (n.status == NodeStatus::Online || n.status == NodeStatus::Busy)
                    && (n.supported_models.iter().any(|m| m == model)
                        || n.role == NodeRole::CloudProxy)
            })
            .cloned()
            .collect()
    }

    /// 取得指定節點
    pub async fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        let map = self.nodes.read().await;
        map.get(node_id).cloned()
    }

    /// 清理心跳逾時之離線節點
    pub async fn clean_stale_nodes(&self, timeout_sec: i64) -> usize {
        let mut map = self.nodes.write().await;
        let now = Utc::now();
        let mut marked_offline = 0;
        for node in map.values_mut() {
            if node.status != NodeStatus::Offline
                && (now - node.last_heartbeat) > Duration::seconds(timeout_sec)
            {
                node.status = NodeStatus::Offline;
                marked_offline += 1;
            }
        }
        marked_offline
    }

    /// 預先注入範例異質節點（方便測試與評估）
    pub async fn seed_sample_cluster(&self) {
        let now = Utc::now();

        // 1. 高階 GPU 工作站 (RTX 4090)
        let rtx4090 = NodeInfo {
            id: "node_workstation_rtx4090".to_string(),
            name: "Lab RTX 4090 Workstation".to_string(),
            role: NodeRole::WorkstationGpu,
            endpoint_url: "http://192.168.1.101:11434".to_string(),
            hardware: NodeHardware {
                gpu_model: Some("NVIDIA GeForce RTX 4090".to_string()),
                total_vram_mb: 24576,
                free_vram_mb: 18432,
                cpu_cores: 16,
                total_ram_mb: 65536,
                free_ram_mb: 48000,
                token_speed_estimate: 75.0,
            },
            supported_models: vec![
                "llama3.2:3b".to_string(),
                "qwen2.5-coder:7b".to_string(),
                "deepseek-r1:8b".to_string(),
                "yolov8-tracking".to_string(),
            ],
            loaded_models: vec!["llama3.2:3b".to_string()],
            current_queue_len: 0,
            network_rtt_ms: 2.5,
            status: NodeStatus::Online,
            last_heartbeat: now,
        };

        // 2. 邊緣 GPU PC (RTX 3060)
        let rtx3060 = NodeInfo {
            id: "node_edge_rtx3060".to_string(),
            name: "Personal RTX 3060 PC".to_string(),
            role: NodeRole::EdgeGpu,
            endpoint_url: "http://192.168.1.102:11434".to_string(),
            hardware: NodeHardware {
                gpu_model: Some("NVIDIA GeForce RTX 3060".to_string()),
                total_vram_mb: 12288,
                free_vram_mb: 8192,
                cpu_cores: 8,
                total_ram_mb: 32768,
                free_ram_mb: 22000,
                token_speed_estimate: 35.0,
            },
            supported_models: vec!["llama3.2:3b".to_string(), "qwen2.5-coder:7b".to_string()],
            loaded_models: vec!["llama3.2:3b".to_string()],
            current_queue_len: 1,
            network_rtt_ms: 4.0,
            status: NodeStatus::Online,
            last_heartbeat: now,
        };

        // 3. 輕量單板電腦 (Raspberry Pi 5)
        let rpi5 = NodeInfo {
            id: "node_sbc_rpi5".to_string(),
            name: "Raspberry Pi 5 Edge SBC".to_string(),
            role: NodeRole::EdgeSbc,
            endpoint_url: "http://192.168.1.105:11434".to_string(),
            hardware: NodeHardware {
                gpu_model: None,
                total_vram_mb: 0,
                free_vram_mb: 0,
                cpu_cores: 4,
                total_ram_mb: 8192,
                free_ram_mb: 5120,
                token_speed_estimate: 8.0,
            },
            supported_models: vec!["llama3.2:1b".to_string(), "qwen2.5:0.5b".to_string()],
            loaded_models: vec!["llama3.2:1b".to_string()],
            current_queue_len: 0,
            network_rtt_ms: 8.5,
            status: NodeStatus::Online,
            last_heartbeat: now,
        };

        // 4. 公有雲代理節點 (Cloud LLM Proxy)
        let cloud_proxy = NodeInfo {
            id: "node_cloud_proxy".to_string(),
            name: "Public Cloud Proxy Endpoint".to_string(),
            role: NodeRole::CloudProxy,
            endpoint_url: "https://api.openai.com".to_string(),
            hardware: NodeHardware {
                gpu_model: Some("Cloud Cluster (H100/A100)".to_string()),
                total_vram_mb: 80000,
                free_vram_mb: 80000,
                cpu_cores: 128,
                total_ram_mb: 512000,
                free_ram_mb: 512000,
                token_speed_estimate: 120.0,
            },
            supported_models: vec![
                "gpt-4o-mini".to_string(),
                "gpt-4o".to_string(),
                "claude-3-5-sonnet".to_string(),
                "llama3.2:3b".to_string(),
            ],
            loaded_models: vec!["gpt-4o-mini".to_string()],
            current_queue_len: 0,
            network_rtt_ms: 65.0,
            status: NodeStatus::Online,
            last_heartbeat: now,
        };

        self.register_or_update(rtx4090).await;
        self.register_or_update(rtx3060).await;
        self.register_or_update(rpi5).await;
        self.register_or_update(cloud_proxy).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = NodeRegistry::new();
        registry.seed_sample_cluster().await;

        let active = registry.list_active_nodes().await;
        assert_eq!(active.len(), 4);

        // 查詢支援 llama3.2:3b 的候選
        let candidates = registry.get_candidates_for_model("llama3.2:3b").await;
        assert!(candidates.len() >= 3);

        // 心跳更新
        let updated = registry.update_heartbeat("node_workstation_rtx4090", 2, 16000, 45000).await;
        assert!(updated);

        let node = registry.get_node("node_workstation_rtx4090").await.unwrap();
        assert_eq!(node.current_queue_len, 2);
    }
}
