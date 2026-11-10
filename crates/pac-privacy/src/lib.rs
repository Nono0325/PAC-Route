pub mod entity_mask;
pub mod gating;
pub mod ldp;
pub mod sketch;

pub use entity_mask::{EntityMaskResult, EntityMasker};
pub use gating::SoftGatingClassifier;
pub use ldp::TwoLayerLdp;
pub use sketch::SemanticSketchPipeline;

#[cfg(test)]
mod tests {
    use super::*;
    use pac_core::types::{ChatMessage, PacInferenceRequest, PrivacyLevel, RoutingMode};

    #[test]
    fn test_entity_mask_and_unmask() {
        let raw = "請寄信給 user@example.com 或撥打 0912-345-678，私鑰密碼是 password: secret_token_123456";
        let masked = EntityMasker::mask(raw);

        assert!(masked.detected_sensitive_count >= 2);
        assert!(!masked.sanitized_text.contains("user@example.com"));
        assert!(!masked.sanitized_text.contains("0912-345-678"));

        let restored = EntityMasker::unmask(&masked.sanitized_text, &masked.entity_map);
        assert_eq!(restored, raw);
    }

    #[test]
    fn test_soft_gating_classification() {
        // 1. 包含敏感機密資料
        let req_secret = PacInferenceRequest::new(
            "llama3.2:3b",
            vec![ChatMessage {
                role: "user".to_string(),
                content: "我的帳號是 admin@nfu.edu.tw，IP 是 140.130.1.1，密碼是 sk-1234567890abcdefghijklmn".to_string(),
            }],
        );
        let (level, mode) = SoftGatingClassifier::classify(&req_secret);
        assert_eq!(level, PrivacyLevel::TopSecret);
        assert_eq!(mode, RoutingMode::DirectLocal);

        // 2. 一般公開問題
        let req_public = PacInferenceRequest::new(
            "llama3.2:3b",
            vec![ChatMessage {
                role: "user".to_string(),
                content: "請解釋什麼是 Rust 的生命週期與所有權機制？".to_string(),
            }],
        );
        let (level_pub, _) = SoftGatingClassifier::classify(&req_public);
        assert_eq!(level_pub, PrivacyLevel::Public);
    }

    #[test]
    fn test_two_layer_ldp_perturbation() {
        let ldp = TwoLayerLdp::new(1.0);
        let noisy = ldp.layer2_laplace_perturb(100.0, 1.0);
        // 拉普拉斯雜訊不等於原始值，但具有合理的數值範圍
        assert!((noisy - 100.0).abs() < 50.0);
    }
}
