# 第四章 演算法設計與 Rust 核心系統實作 (Algorithms & Rust Implementation)

本章將從工程實踐角度，詳細剖析 PAC-Route 之軟體模組結構、Tokio 非同步管線設計、三模式軟體閘門與雙層 LDP 實作、Dueling DDQN 排程器程式碼架構，以及相容 OpenAI/Ollama 協定之轉發引擎。

---

## 4.1 系統模組架構與 Cargo Workspace 劃分

為確保高內聚力、低耦合性以及卓越的編譯與除錯效率，PAC-Route 採用 Cargo Workspace 架構進行多 Crate 劃分：

```
pac-route-workspace/
├── crates/
│   ├── pac-core/          # 核心領域模型、SLA 數學模型、Trait 與錯誤定義
│   ├── pac-discovery/     # mDNS-SD / UDP 局域網節點自動發現與註冊表
│   ├── pac-scheduler/     # Dueling DDQN 調度演算法與 SLA 動作遮罩
│   ├── pac-privacy/       # 實體遮蔽、2-Layer LDP 擾動與語義草圖管線
│   ├── pac-router/        # Axum 高效能非同步 HTTP 代理核心與 OpenAI 相容端點
│   └── pac-node-agent/    # 部署於邊緣設備之輕量級系統資源監控守護進程
```

各 Crate 透過 Workspace 共享依賴版本，並遵循嚴格的依賴方向（`pac-core` 不依賴任何業務模組，`pac-router` 聚合各模組）。

---

## 4.2 非同步非阻塞網路代理核心 (Axum + Tokio)

PAC-Route 使用最新版 `axum` 與 `tokio` 作為底層非同步框架。相較於 Python 的 Tornado 或 FastAPI，Rust 的非同步執行期完全擺脫了 GIL 束縛，能跨多核心平攤微小連線開銷：

```rust
// crates/pac-router/src/main.rs (節選)
let app = Router::new()
    .route("/health", get(health_handler))
    .route("/cluster/nodes", get(cluster_nodes_handler))
    .route("/v1/models", get(list_models_handler))
    .route("/v1/chat/completions", post(chat_completions_handler))
    .route("/api/chat", post(chat_completions_handler)) // Ollama 相容
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .with_state(app_state);

let listener = tokio::net::TcpListener::bind(bind_addr).await?;
axum::serve(listener, app).await?;
```

每個抵達的 HTTP 請求在 `tokio` 輕量協程（Task）中被處理，記憶體開銷僅數 KB，具備支撐數萬並發請求之能力。

---

## 4.3 動態軟體閘門與雙層 LDP 實作

### 1. 敏感實體辨識與遮蔽 (EntityMasker)
系統採用高效正規表達式管線對傳入文字進行掃描，針對 Email、IP 位址、API 密鑰與手機號碼進行正則擷取並置換為專屬 Mask 標籤：

```rust
// crates/pac-privacy/src/entity_mask.rs (核心實作)
pub fn mask(text: &str) -> EntityMaskResult {
    let mut entity_map = HashMap::new();
    let mut sanitized = text.to_string();
    let mut counter = 0;

    // 匹配並置換 API Key / Secret / Token
    let key_regex = Regex::new(
        r"(?i)(sk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{20,}|password\s*[:=]\s*\S+)"
    ).unwrap();
    
    for mat in key_regex.find_iter(&sanitized.clone()) {
        counter += 1;
        let placeholder = format!("[MASKED_SECRET_{}]", counter);
        entity_map.insert(placeholder.clone(), mat.as_str().to_string());
        sanitized = sanitized.replace(mat.as_str(), &placeholder);
    }
    // ... 同步進行 Email, IP, 電話之提取 ...
    EntityMaskResult { sanitized_text: sanitized, entity_map, detected_sensitive_count: counter }
}
```

### 2. 雙層差分隱私注入 (TwoLayerLdp)
在數值及特徵統計發布上，利用逆轉換採樣（Inverse Transform Sampling）生成符合拉普拉斯分佈之擾動雜訊：

```rust
// crates/pac-privacy/src/ldp.rs
pub fn layer2_laplace_perturb(&self, true_value: f64, sensitivity: f64) -> f64 {
    let mut rng = rand::thread_rng();
    let scale = sensitivity / self.epsilon.max(0.01);
    let u: f64 = rng.gen_range(-0.5..0.5);
    let sign = if u < 0.0 { -1.0 } else { 1.0 };
    let noise = -scale * sign * (1.0 - 2.0 * u.abs()).ln();
    true_value + noise
}
```

---

## 4.4 Dueling DDQN SLA 動作遮罩調度器之實作

排程核心模組封裝了前瞻性延遲預測器與動作遮罩邏輯。在評估候選節點時，程式先計算 $T_{total}$ 與隱私約束：

```rust
// crates/pac-scheduler/src/dueling_ddqn.rs (關鍵決策區塊)
for node in candidates {
    let latency = LatencyPredictor::predict(request, node);
    let mut is_masked = false;
    let mut mask_reason = None;

    // SLA 期限硬性遮罩檢查
    if request.sla.strict_action_masking && latency.total_ms > request.sla.deadline_ms {
        is_masked = true;
        mask_reason = Some(format!("預估耗時 {:.1}ms 超過 SLA Deadline {:.1}ms", 
                                   latency.total_ms, request.sla.deadline_ms));
    }

    // 隱私合規性遮罩檢查：嚴格禁止未授權之 TopSecret 上雲
    if node.role == NodeRole::CloudProxy && effective_privacy == PrivacyLevel::TopSecret {
        is_masked = true;
        mask_reason = Some("機密資料強制禁止路由至雲端".to_string());
    }

    // 計算 Q 值 (被遮蔽之動作給予極大負懲罰)
    let q_value = if is_masked {
        -self.sla_violation_penalty
    } else {
        let base_v = -100.0;
        let advantage = -(latency.total_ms) + (node.hardware.token_speed_estimate * 2.0);
        base_v + advantage
    };
    evaluations.push((node.clone(), ActionEvaluation { node_id: node.id.clone(), is_masked, mask_reason, predicted_total_ms: latency.total_ms, q_value }));
}

// 選取非遮蔽動作中 Q 值最高者
let valid_actions: Vec<_> = evaluations.iter().filter(|(_, eval)| !eval.is_masked).collect();
```

這項實作確保了系統決策完全受限於 SLA 與隱私邊界，絕不將違規動作付諸執行。

---

## 4.5 相容標準 API 介面實現

為了讓 PAC-Route 可以作為既有 AI 應用程式（如 VS Code 擴充套件、Cursor、Open-WebUI、LangChain 等）的透明代理，系統實作了與 OpenAI `/v1/chat/completions` 完全一致的 JSON 協定。

同時，PAC-Route 透過自訂 HTTP Response Headers 注入監控資訊：
* `X-PAC-Node-Id`：實際執行該推論之異質節點代號；
* `X-PAC-Routing-Mode`：選定之路由模式（`DirectLocal`、`DirectCloud` 或 `SemanticSketchCollaborative`）；
* `X-PAC-Privacy-Level`：自動檢測出之隱私防護等級。

此外，針對進階開發者，PAC-Route 允許客戶端在 Request Headers 中直接附加 `X-SLA-Deadline-Ms: 1500` 或 `X-Privacy-Level: TopSecret`，以達成程式級別之精準服務級別控制。

