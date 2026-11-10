use std::collections::HashMap;
use crate::entity_mask::EntityMasker;
use crate::ldp::TwoLayerLdp;

/// 語義草圖協同處理器 (Semantic Sketch Collaborator)
pub struct SemanticSketchPipeline {
    pub ldp: TwoLayerLdp,
}

impl Default for SemanticSketchPipeline {
    fn default() -> Self {
        Self {
            ldp: TwoLayerLdp::default(),
        }
    }
}

impl SemanticSketchPipeline {
    pub fn new(epsilon: f64) -> Self {
        Self {
            ldp: TwoLayerLdp::new(epsilon),
        }
    }

    /// 產生脫敏後的語義草圖請求 (傳送至雲端之無隱私 Prompt)
    pub fn prepare_sketch_prompt(&self, original_prompt: &str) -> (String, HashMap<String, String>) {
        let mask_res = EntityMasker::mask(original_prompt);

        // 構造雲端語義草圖指導提示詞 (引導雲端模型保留標籤占位符)
        let sketch_prompt = format!(
            "你是一個高效的結構草圖生成器。請依據以下經過實體遮蔽（如 [MASKED_...]）的內容，生成邏輯結構完整的分析或回答草圖。\n\
            請務必原樣保留所有 [MASKED_...] 標籤以供本地客戶端還原。\n\n\
            【輸入內容】：\n{}",
            mask_res.sanitized_text
        );

        (sketch_prompt, mask_res.entity_map)
    }

    /// 本地端重構：將雲端回傳的抽象草圖，結合本地私密對照表無縫還原
    pub fn reconstruct_final_output(
        &self,
        cloud_sketch_response: &str,
        entity_map: &HashMap<String, String>,
    ) -> String {
        EntityMasker::unmask(cloud_sketch_response, entity_map)
    }
}
