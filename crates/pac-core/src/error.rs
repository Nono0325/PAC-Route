// 錯誤類別定義
#[derive(thiserror::Error, Debug)]
pub enum PacError {
    #[error("SLA Deadline 超時違約：預估或實際耗時 {actual_ms}ms 超過上限 {deadline_ms}ms")]
    SlaDeadlineExceeded { actual_ms: f64, deadline_ms: f64 },

    #[error("隱私約束違規：機密等級為 {level:?}，未獲用戶授權傳送至外部雲端")]
    PrivacyViolation { level: String },

    #[error("無合適之可用節點：所有節點皆被 SLA 動作遮罩或離線")]
    NoEligibleNodeFound,

    #[error("節點不可用：節點 {node_id} 回應超時或處於離線狀態")]
    NodeUnavailable { node_id: String },

    #[error("語義草圖重構失敗：{reason}")]
    SketchReconstructionFailed { reason: String },

    #[error("網路傳輸錯誤：{0}")]
    NetworkError(String),

    #[error("序列化或解析錯誤：{0}")]
    SerializationError(String),

    #[error("內部系統錯誤：{0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, PacError>;
