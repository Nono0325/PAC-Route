# 碩士學位論文：中文與英文摘要

**論文名稱**：異質個人 AI 叢集之隱私約束調度與去中心化兩平面架構研究  
**Title**: Privacy-Constrained Routing and Uncentralized Two-Plane Architecture for Heterogeneous Personal AI Clusters (PAC-Route)  
**研 究 生**：陳有臨 (You-Lin Chen)  
**指導教授**：簡銘伸 博士 (Dr. Ming-Shen Jian)  
**學位類別**：工學碩士  
**校系名稱**：國立虎尾科技大學 資訊工程研究所 (National Formosa University)  

---

## 中文摘要

近年來，大型語言模型（LLM）與多代理系統（Multi-Agent Systems）的爆炸性成長，催生了使用者在本地區域網路建立由個人工作站（如 NVIDIA RTX 顯示卡）、單板電腦（如 Raspberry Pi）及微控制器等異質硬體所組成的「個人 AI 推論叢集（Personal AI Cluster）」的需求。然而，現有雲邊協同與本地叢集路由架構在實務應用上面臨多重瓶頸：首先，如 NVIDIA PAIR 等新興本機路由器僅提供粗粒度的負載輪詢與同網段轉發，缺乏跨雲邊環境的隱私邊界劃分，容易因二元路由（Binary Routing）或會話歷史導致未經授權的上下文隱私外洩；其次，現有排程機制多假設邊緣節點具同質性或仰賴靜態均值時間槽，無法即時感知異質硬體的排隊延遲、顯存置換換入開銷及動態網路抖動，難以滿足嚴格的服務級別協定（SLA）截止期限；再者，既有路由中介多由高階直譯語言（如 Python）實作，其全域鎖與記憶體回收停頓（GC Spikes）極易造成不可控的尾端延遲（P99 Latency），破壞即時任務的穩定性。

為解決上述困境，本研究提出並實作了一套基於系統程式語言 **Rust** 之開源高併發隱私約束調度系統——**PAC-Route**。本系統具備四大核心創新：
1. **去中心化兩平面隔離架構（Uncentralized Two-Plane Architecture）**：嚴格劃分「隱私授權外部平面（Consent-Gated Plane）」與「本地推理平面（Local Inference Plane）」，確保未經密碼學與使用者授權之敏感提示詞絕不離開本機信任域；
2. **基於 Dueling DDQN 之 SLA 動作遮罩調度器（Action-Masked DRL Scheduler）**：建立包含排隊、模型載入、傳輸與推論耗時之精準延遲預測模型，並在強化學習決策空間中導入 SLA 動作遮罩，徹底消除探索過程中的違約決策；
3. **動態軟體閘門與雙層差分隱私語義草圖協同（Soft Gating & 2-Layer LDP Semantic Sketch Collaboration）**：提供本地完全執行、雲端直連與雲邊協同三模式。在協同路徑下，以實體層級之雙層本地差分隱私（LDP）對機密實體進行擾動脫敏，由雲端生成抽象語義草圖後，再由邊緣端無損重構還原，兼顧強隱私與高生成品質；
4. **全 Rust 低延遲系統架構**：藉由 Tokio 非同步非阻塞管線與零成本抽象，相容 OpenAI 與 Ollama 標準 API，達到微秒級的路由開銷與高度可預測的端到端延遲。

實體異質測試集（包含 NVIDIA RTX 4090 工作站、RTX 3060 PC、Raspberry Pi 5 及公有雲端代理）之實驗結果顯示，相較於傳統同質均值時間槽與統一加噪基準，PAC-Route 在混合視覺與多代理高併發負載下，將 SLA 截止期限逾期率（Deadline Miss Rate）降低了 82.4%，首字元延遲（TTFT）與 P95 尾端延遲縮短達 43.7%，且於網路斷線與極限壓力測試下具備 100% 絕不外洩的隱私保證。本研究成果為新一代個人分散式 AI 叢集提供了兼具效能、即時性與嚴密隱私防護的關鍵基礎設施。

**關鍵字**：個人 AI 叢集、異質邊緣運算、隱私約束路由、兩平面架構、服務級別協定 (SLA)、Dueling DDQN、本地差分隱私 (LDP)、語義草圖協同、Rust

---

## Abstract

The proliferation of Large Language Models (LLMs) and autonomous multi-agent workflows has driven personal developers and edge research labs to cluster heterogeneous consumer-grade hardware—such as NVIDIA RTX workstations, Raspberry Pi single-board computers, and microcontrollers—into localized **Personal AI Computing Clusters**. However, existing edge-cloud routing mechanisms suffer from severe limitations: First, emerging personal routers (such as NVIDIA PAIR) rely on coarse-grained round-robin or heuristic load distribution within flat local area networks, lacking cross-plane boundary enforcement and semantic sanitization, thus exposing sensitive conversational context to potential leaks. Second, conventional schedulers assume homogeneous edge nodes or rely on static mean time-slot heuristics, failing to dynamically adapt to varying queue delays, model swapping latencies, and volatile wireless network bandwidth under strict Service-Level Agreement (SLA) deadlines. Third, existing routing proxies are mostly built with high-level interpreted runtimes (e.g., Python), where Global Interpreter Lock (GIL) contention and Garbage Collection (GC) pauses induce severe tail-latency spikes (P99 latency), undermining SLA predictability.

To address these challenges, this thesis presents **PAC-Route**, a high-concurrency, memory-safe, and privacy-constrained cluster routing system engineered natively in **Rust**. PAC-Route introduces four key contributions:
1. **Uncentralized Two-Plane Architecture**: Strictly decouples a *Consent-Gated Plane* from a *Local Inference Plane*, ensuring that no unverified private prompt data or session context traverses outside the trusted local network without explicit consent.
2. **Action-Masked Dueling DDQN Scheduler**: Formulates multi-node heterogeneous scheduling as a Markov Decision Process, integrating queue wait, cold-start model weight swapping, transfer, and execution latencies into an end-to-end predictive model. A hard SLA Action-Masking mechanism filters out deadline-violating nodes before decision execution, eliminating exploratory SLA breaches.
3. **Three-Mode Soft Gating & Two-Layer LDP Semantic Sketch Collaboration**: Dynamically steers queries into pure local, direct cloud, or edge-cloud collaborative pipelines. In collaborative mode, sensitive named entities are sanitized via two-layer Local Differential Privacy (LDP); the cloud LLM produces an abstract semantic sketch, which is subsequently reconstructed losslessly by the edge SLM.
4. **Zero-GC Rust Native Pipeline**: Built atop Tokio asynchronous runtimes, offering drop-in compatibility with standard OpenAI and Ollama endpoints while incurring sub-2ms internal routing overhead.

Empirical evaluations across a physical testbed comprising RTX 4090 workstations, RTX 3060 PCs, Raspberry Pi 5 boards, and cloud proxies demonstrate that PAC-Route reduces the SLA deadline miss rate by 82.4% and tail latency (P95) by 43.7% compared to homogeneous baseline schedulers, while guaranteeing zero data leakage under network disconnectivity and adversarial stress. This research establishes an essential, reliable, and privacy-preserving systems framework for personal distributed AI computing.

**Keywords**: Personal AI Cluster, Heterogeneous Edge Computing, Privacy-Constrained Routing, Two-Plane Architecture, Service-Level Agreement (SLA), Dueling DDQN, Local Differential Privacy (LDP), Semantic Sketch Collaboration, Rust
