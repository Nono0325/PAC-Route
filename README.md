# PAC-Route: 異質個人 AI 叢集之隱私約束調度與兩平面架構

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/Status-Thesis%20Ready-green.svg)]()

> **論文名稱**：異質個人 AI 叢集之隱私約束調度與去中心化兩平面架構研究  
> **Title**: Privacy-Constrained Routing and Uncentralized Two-Plane Architecture for Heterogeneous Personal AI Clusters  
> **研 究 生**：陳有臨 (You-Lin Chen)  
> **指導教授**：簡銘伸 博士 (Dr. Ming-Shen Jian)  
> **研究單位**：國立虎尾科技大學 資訊工程研究所 (NFU CSIE)  
> **開源代碼倉庫**：[https://github.com/Nono0325/PAC-Route.git](https://github.com/Nono0325/PAC-Route.git)

---

## 📖 系統簡介 (Overview)

**PAC-Route** 是一套以系統程式語言 **Rust** 打造、專為個人開發者與邊緣研究室設計的高效能、低延遲、隱私約束推論調度中介系統。

當使用者利用區域網路內的異質設備（如 NVIDIA RTX 4090 工作站、RTX 3060 PC、Raspberry Pi 5、微控制器）組建私有 AI 叢集時，PAC-Route 能自動發現節點算力，並在保證服務級別協定（SLA）截止期限與資料機密性的前提下，將推論任務智慧分流至最佳節點或雲邊協同管線。

```mermaid
flowchart TD
    Client[Cursor / VSCode / WebUI] -->|OpenAI-Compatible API| Gateway[PAC-Route 閘道器 (Port 8088)]
    
    subgraph Routing ["核心決策模組"]
        Gateway --> Gating[動態軟體閘門 Soft Gating]
        Gating --> ModeLocal[模式 1: 完全本地推論]
        Gating --> ModeCloud[模式 2: 純非敏感直連雲端]
        Gating --> ModeSketch[模式 3: 雙層 LDP 語義草圖協同]
        
        ModeLocal --> Scheduler[Dueling DDQN 排程器<br>+ SLA 動作遮罩]
    end
    
    subgraph Heterogeneous_Cluster ["局域網異質節點 (Local LAN)"]
        Scheduler --> N1[RTX 4090 PC (Ollama / vLLM)]
        Scheduler --> N2[RTX 3060 PC (LM Studio)]
        Scheduler --> N3[Raspberry Pi 5 (輕量嵌入/還原)]
    end
```

---

## 🚀 四大核心技術特點

1. **去中心化兩平面隔離 (Uncentralized Two-Plane Architecture)**：
   劃分「本地推理平面」與「隱私授權外部平面」，未經明確密碼學簽署授權之資料絕不靜默外洩至外部公網。
2. **基於 Dueling DDQN 之 SLA 動作遮罩調度器 (SLA Action-Masking)**：
   聯合排隊延遲（$T_{queue}$）、模型載入開銷（$T_{load}$）、網路延遲（$T_{trans}$）與執行耗時（$T_{exec}$）。動作遮罩機制直接排除預估逾期節點，將 SLA 違約率降低 **82.4%**。
3. **動態軟體閘門與雙層本地差分隱私 (2-Layer LDP Semantic Sketch)**：
   自動提取 PII、金鑰與敏感實體，雲端僅生成抽象語義草圖，本地 SLM 實施無失真還原，機密實體外洩率為 **0.0%**。
4. **全 Rust 原生高效實作 (Zero GC)**：
   Tokio + Axum 非同步架構，完全向下相容 OpenAI (`/v1/chat/completions`) 與 Ollama (`/api/chat`) 端點，內建 mDNS/UDP 局域網節點自動發現。

---

## 📂 專案模組架構 (Cargo Workspace)

```
PAC-Route/
├── Cargo.toml                    # 工作區配置 (Workspace Root)
├── crates/
│   ├── pac-core/                 # 核心領域模型、SLA 數學模型與 Trait 定義
│   ├── pac-discovery/            # 局域網 mDNS / UDP 自動能力發現與註冊表
│   ├── pac-scheduler/            # Dueling DDQN 排程演算法與 SLA 動作遮罩
│   ├── pac-privacy/              # 實體遮蔽、2-Layer LDP 與語義草圖協同
│   ├── pac-router/               # Axum HTTP/mTLS 代理服務器 (Port 8088)
│   └── pac-node-agent/           # 部署在邊緣端 (樹莓派/PC) 的輕量監控 Agent
├── docs/
│   └── thesis/                   # 碩士學位論文完整稿件 (第一章至第六章與參考文獻)
│       ├── 00_ABSTRACT.md
│       ├── 01_INTRODUCTION.md
│       ├── 02_RELATED_WORK.md
│       ├── 03_SYSTEM_ARCHITECTURE.md
│       ├── 04_RUST_IMPLEMENTATION.md
│       ├── 05_EVALUATION.md
│       ├── 06_CONCLUSION.md
│       └── REFERENCES.md
```

---

## 🛠️ 編譯與運行指引 (Quick Start)

### 1. 環境需求
* [Rust](https://rustup.rs/) (建議版本 $\ge 1.75$)
* Cargo 套件管理器

### 2. 執行全專案單元測試與排程驗證
```powershell
cargo test --workspace
```

### 3. 啟動 PAC-Route 叢集閘道器
```powershell
cargo run -p pac-router
```
預設閘道服務將監聽於 `http://0.0.0.0:8088`：
* **健康狀態檢查**：`GET http://localhost:8088/health`
* **叢集節點查詢**：`GET http://localhost:8088/cluster/nodes`
* **模型清單 (OpenAI 相容)**：`GET http://localhost:8088/v1/models`
* **推論端點 (OpenAI 相容)**：`POST http://localhost:8088/v1/chat/completions`

### 4. 測試推論請求 (支援 SLA 與隱私標頭)
```powershell
curl -X POST http://localhost:8088/v1/chat/completions `
  -H "Content-Type: application/json" `
  -H "X-SLA-Deadline-Ms: 2500" `
  -H "X-Privacy-Level: TopSecret" `
  -d '{
    "model": "llama3.2:3b",
    "messages": [
      {"role": "user", "content": "請分析本機機密設定檔，我的私鑰是 sk-999988887777"}
    ]
  }'
```

---

## 📜 論文全文導引 (Master's Thesis Outline)

本專案之碩士學位論文稿件存放於 `docs/thesis/` 目錄中：
1. [00_ABSTRACT.md](docs/thesis/00_ABSTRACT.md) - 中英文摘要與關鍵字
2. [01_INTRODUCTION.md](docs/thesis/01_INTRODUCTION.md) - 第一章 緒論
3. [02_RELATED_WORK.md](docs/thesis/02_RELATED_WORK.md) - 第二章 文獻探討與相關技術
4. [03_SYSTEM_ARCHITECTURE.md](docs/thesis/03_SYSTEM_ARCHITECTURE.md) - 第三章 PAC-Route 系統架構與數學建模
5. [04_RUST_IMPLEMENTATION.md](docs/thesis/04_RUST_IMPLEMENTATION.md) - 第四章 演算法設計與 Rust 核心系統實作
6. [05_EVALUATION.md](docs/thesis/05_EVALUATION.md) - 第五章 實驗設計與效能評估
7. [06_CONCLUSION.md](docs/thesis/06_CONCLUSION.md) - 第六章 結論與未來研究方向
8. [REFERENCES.md](docs/thesis/REFERENCES.md) - 參考文獻
