use rand::Rng;

/// 雙層本地差分隱私 (Two-Layer Local Differential Privacy, 2-Layer LDP)
pub struct TwoLayerLdp {
    /// 隱私預算 epsilon (越小隱私保護越高，雜訊越大)
    pub epsilon: f64,
}

impl Default for TwoLayerLdp {
    fn default() -> Self {
        Self { epsilon: 1.5 }
    }
}

impl TwoLayerLdp {
    pub fn new(epsilon: f64) -> Self {
        Self { epsilon }
    }

    /// 第一層：隨機化回應 (Randomized Response) 針對實體類別進行翻轉擾動
    /// 滿足 e^epsilon / (1 + e^epsilon) 機率保持真實，其餘隨機擾動
    pub fn layer1_randomized_response(&self, is_sensitive: bool) -> bool {
        let mut rng = rand::thread_rng();
        let p = self.epsilon.exp() / (1.0 + self.epsilon.exp());
        let roll: f64 = rng.gen();
        if roll <= p {
            is_sensitive
        } else {
            !is_sensitive
        }
    }

    /// 第二層：拉普拉斯機制 (Laplace Mechanism) 注入連續空間數值擾動
    /// 用於統計數據、數值特徵或延遲估算的差分隱私發布
    pub fn layer2_laplace_perturb(&self, true_value: f64, sensitivity: f64) -> f64 {
        let mut rng = rand::thread_rng();
        let scale = sensitivity / self.epsilon.max(0.01);
        let u: f64 = rng.gen_range(-0.5..0.5);
        let sign = if u < 0.0 { -1.0 } else { 1.0 };
        let noise = -scale * sign * (1.0 - 2.0 * u.abs()).ln();
        true_value + noise
    }
}
