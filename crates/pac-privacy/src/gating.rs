use pac_core::types::{PacInferenceRequest, PrivacyLevel, RoutingMode};
use crate::entity_mask::EntityMasker;

/// 三模式軟體閘門 (Soft Gating Module)
pub struct SoftGatingClassifier;

impl SoftGatingClassifier {
    /// 分析請求內容，決定隱私等級與最佳路由模式
    pub fn classify(request: &PacInferenceRequest) -> (PrivacyLevel, RoutingMode) {
        // 1. 若請求中已經顯式宣告隱私等級，直接遵從顯式等級
        if let Some(level) = request.explicit_privacy_level {
            let mode = match level {
                PrivacyLevel::TopSecret => RoutingMode::DirectLocal,
                PrivacyLevel::Restricted => {
                    if request.user_consent_for_cloud {
                        RoutingMode::SemanticSketchCollaborative
                    } else {
                        RoutingMode::DirectLocal
                    }
                }
                PrivacyLevel::Public => {
                    if request.user_consent_for_cloud {
                        RoutingMode::DirectCloud
                    } else {
                        RoutingMode::DirectLocal
                    }
                }
            };
            return (level, mode);
        }

        // 2. 自動敏感實體掃描
        let prompt = request.prompt_text();
        let mask_res = EntityMasker::mask(&prompt);

        // 計算敏感度評分 (0.0 ~ 1.0)
        let sensitivity_score = if mask_res.detected_sensitive_count >= 3 {
            0.95
        } else if mask_res.detected_sensitive_count >= 1 {
            0.55
        } else {
            0.05
        };

        if sensitivity_score >= 0.8 {
            (PrivacyLevel::TopSecret, RoutingMode::DirectLocal)
        } else if sensitivity_score >= 0.3 {
            (
                PrivacyLevel::Restricted,
                RoutingMode::SemanticSketchCollaborative,
            )
        } else {
            // 純公開且無敏感資訊
            if request.user_consent_for_cloud {
                (PrivacyLevel::Public, RoutingMode::DirectCloud)
            } else {
                // 預設兩平面防護原則：無明確外部授權時優先於本機網段推論
                (PrivacyLevel::Public, RoutingMode::DirectLocal)
            }
        }
    }
}
