use regex::Regex;
use std::collections::HashMap;

/// 脫敏遮蔽與對照映射表
#[derive(Debug, Clone, Default)]
pub struct EntityMaskResult {
    pub sanitized_text: String,
    pub entity_map: HashMap<String, String>,
    pub detected_sensitive_count: usize,
}

pub struct EntityMasker;

impl EntityMasker {
    /// 偵測並替換敏感個人資訊 (PII / Secrets / Tokens)
    pub fn mask(text: &str) -> EntityMaskResult {
        let mut entity_map = HashMap::new();
        let mut sanitized = text.to_string();
        let mut counter = 0;

        // 1. Email 匹配與替換
        let email_regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
        for mat in email_regex.find_iter(text) {
            counter += 1;
            let placeholder = format!("[MASKED_EMAIL_{}]", counter);
            entity_map.insert(placeholder.clone(), mat.as_str().to_string());
            sanitized = sanitized.replace(mat.as_str(), &placeholder);
        }

        // 2. IP 地址匹配
        let ip_regex = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap();
        for mat in ip_regex.find_iter(&sanitized.clone()) {
            // 排除常規 local 掩蔽以防誤判
            if mat.as_str() != "0.0.0.0" && mat.as_str() != "127.0.0.1" {
                counter += 1;
                let placeholder = format!("[MASKED_IP_{}]", counter);
                entity_map.insert(placeholder.clone(), mat.as_str().to_string());
                sanitized = sanitized.replace(mat.as_str(), &placeholder);
            }
        }

        // 3. API Key / Token (如 sk-xxx, Bearer xxx, ghp_xxx)
        let key_regex = Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{20,}|password\s*[:=]\s*\S+)").unwrap();
        for mat in key_regex.find_iter(&sanitized.clone()) {
            counter += 1;
            let placeholder = format!("[MASKED_SECRET_{}]", counter);
            entity_map.insert(placeholder.clone(), mat.as_str().to_string());
            sanitized = sanitized.replace(mat.as_str(), &placeholder);
        }

        // 4. 電話號碼 (台灣手機與一般格式)
        let phone_regex = Regex::new(r"\b09\d{2}-?\d{3}-?\d{3}\b").unwrap();
        for mat in phone_regex.find_iter(&sanitized.clone()) {
            counter += 1;
            let placeholder = format!("[MASKED_PHONE_{}]", counter);
            entity_map.insert(placeholder.clone(), mat.as_str().to_string());
            sanitized = sanitized.replace(mat.as_str(), &placeholder);
        }

        EntityMaskResult {
            sanitized_text: sanitized,
            entity_map,
            detected_sensitive_count: counter,
        }
    }

    /// 將雲端生成之草圖中的 Masked 標籤重構還原為本地私有真實值
    pub fn unmask(sketch_text: &str, entity_map: &HashMap<String, String>) -> String {
        let mut restored = sketch_text.to_string();
        for (placeholder, original) in entity_map {
            restored = restored.replace(placeholder, original);
        }
        restored
    }
}
