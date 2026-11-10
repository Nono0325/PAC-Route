use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 節點硬體角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// 高階 GPU 工作站 (如 RTX 4090/3090，可載入大模型)
    WorkstationGpu,
    /// 中階邊緣 PC (如 RTX 3060/Laptop GPU)
    EdgeGpu,
    /// 輕量單板電腦 (如 Raspberry Pi 5 / Jetson Nano，執行小模型或草圖還原)
    EdgeSbc,
    /// 微控制器邊緣閘道 (如 ESP32，僅做傳感與轉發)
    MicroController,
    /// 公有雲 Proxy (如 OpenAI / Anthropic / Groq)
    CloudProxy,
}

/// 節點硬體資源描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHardware {
    pub gpu_model: Option<String>,
    pub total_vram_mb: u64,
    pub free_vram_mb: u64,
    pub cpu_cores: usize,
    pub total_ram_mb: u64,
    pub free_ram_mb: u64,
    /// 估算推論速度 (tokens/sec)
    pub token_speed_estimate: f64,
}

impl Default for NodeHardware {
    fn default() -> Self {
        Self {
            gpu_model: None,
            total_vram_mb: 0,
            free_vram_mb: 0,
            cpu_cores: 4,
            total_ram_mb: 8192,
            free_ram_mb: 4096,
            token_speed_estimate: 15.0,
        }
    }
}

/// 節點運行狀態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Busy,
    Draining,
    Offline,
}

/// 叢集節點完整資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub role: NodeRole,
    pub endpoint_url: String,
    pub hardware: NodeHardware,
    pub supported_models: Vec<String>,
    pub loaded_models: Vec<String>,
    pub current_queue_len: usize,
    pub network_rtt_ms: f64,
    pub status: NodeStatus,
    pub last_heartbeat: DateTime<Utc>,
}

/// 隱私等級標籤 (Privacy Classification Level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// 完全公開資料 (可自由導向雲端)
    Public = 0,
    /// 受限資料 (可經過雙層 LDP 語義草圖脫敏後協同)
    Restricted = 1,
    /// 機密資料 (必須留存於局域網完全本機推論，絕不出 LAN)
    TopSecret = 2,
}

/// 路由執行模式 (三模式軟體閘門 Soft Gating Decision)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingMode {
    /// 模式 1：完全本機推論 (Local Inference Plane)
    DirectLocal,
    /// 模式 2：直接雲端推論 (Consent-Gated Plane - Public Only)
    DirectCloud,
    /// 模式 3：雲邊語義草圖協同 (Semantic Sketch Collaboration with 2-Layer LDP)
    SemanticSketchCollaborative,
}

/// 服務級別協定 (SLA Policy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaPolicy {
    /// 任務截止時間 (Deadline in milliseconds)
    pub deadline_ms: f64,
    /// 是否啟動硬性動作遮罩 (True 則超過 Deadline 直接排除該節點)
    pub strict_action_masking: bool,
    /// 最大允許之排隊等待時間 (Max queue wait ms)
    pub max_queue_wait_ms: f64,
}

impl Default for SlaPolicy {
    fn default() -> Self {
        Self {
            deadline_ms: 3000.0,
            strict_action_masking: true,
            max_queue_wait_ms: 1000.0,
        }
    }
}

/// 推論請求訊息規格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 進入 PAC-Route 的推論請求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacInferenceRequest {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stream: bool,
    pub sla: SlaPolicy,
    pub explicit_privacy_level: Option<PrivacyLevel>,
    /// 是否由使用者明確簽署外部傳輸授權 (Consent-Gated)
    pub user_consent_for_cloud: bool,
    pub created_at: DateTime<Utc>,
}

impl PacInferenceRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            id: format!("req_{}", Uuid::new_v4().simple()),
            messages,
            model: model.into(),
            max_tokens: Some(512),
            temperature: Some(0.7),
            stream: false,
            sla: SlaPolicy::default(),
            explicit_privacy_level: None,
            user_consent_for_cloud: false,
            created_at: Utc::now(),
        }
    }

    /// 取得最後一則 Prompt 的純文字
    pub fn prompt_text(&self) -> String {
        self.messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default()
    }

    /// 粗估輸入 Token 總數 (中英文混合估算)
    pub fn estimate_token_count(&self) -> usize {
        let total_chars: usize = self.messages.iter().map(|m| m.content.chars().count()).sum();
        // 粗略以每 2 個字符約為 1 Token 估計
        (total_chars / 2).max(1)
    }
}

/// 延遲組成拆解指標
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyBreakdown {
    /// 佇列等待時間 (ms)
    pub queue_ms: f64,
    /// 網路傳輸延遲 (ms)
    pub trans_ms: f64,
    /// 模型冷啟動 / 載入延遲 (ms)
    pub load_ms: f64,
    /// 實際推論執行時間 (ms)
    pub exec_ms: f64,
    /// 總端到端耗時 (ms)
    pub total_ms: f64,
}

/// 推論回應規格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacInferenceResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub routing_mode: RoutingMode,
    pub routed_node_id: String,
    pub latency: LatencyBreakdown,
    pub privacy_level_detected: PrivacyLevel,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub created_at: DateTime<Utc>,
}
