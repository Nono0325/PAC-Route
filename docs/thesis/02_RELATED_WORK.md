# 第二章 文獻探討與相關技術 (Related Work)

本章針對異質個人 AI 叢集路由、雲邊協同推論中的隱私保護、基於強化學習的服務級別協定（SLA）調度演算法，以及低延遲系統程式語言 Rust 進行深入之文獻探討與技術脈絡梳理。

---

## 2.1 本地個人 AI 叢集與邊緣推論架構 (Local AI Clusters & Edge Inference)

傳統分散式推論多建立於超大規模資料中心（Hyperscale Data Centers）之集中式叢集（如 Kubernetes 搭配 Ray, Triton Inference Server 或 vLLM 叢集）。這類架構依賴高頻寬、低延遲的 InfiniBand 網路與同質高階 GPU（如 NVIDIA H100/A100），在硬體採購預算與能耗上遠非個人開發者或邊緣實驗室所能承擔。

隨著開源輕量化模型（如 Llama 3、Qwen 2.5、DeepSeek-R1）的成熟，在消費級硬體上運行量化模型（GGUF / AWQ）的本機推論引擎（如 Ollama、LM Studio、llama.cpp）獲得極高普及率。然而，單機推論引擎缺乏多節點擴展能力。

在 2026 年初，NVIDIA 正式開源推出了 **Personal AI Router (PAIR)**。PAIR 旨在利用使用者同網段內的閒置設備（Windows PC、Mac mini、RTX 工作站等），透過 mDNS 廣播與相容 OpenAI/Ollama 協定進行平行多代理任務的負載分發。PAIR 確立了「**不進行模型切分（No Tensor Parallelism/Sharding across machines），而是針對獨立請求進行 Scale-out 分流**」的架構原則，極大降低了網路通訊對跨機張量同步的硬體依賴。

然而，現有類似 PAIR 之架構存在根本性限制：
1. **拓撲封閉性與雲邊隔離**：PAIR 假定所有請求均能在局域網內消化，一旦本機算力耗盡，無法安全地將任務卸載至雲端；
2. **缺乏上下文敏感度分析**：未對輸入提示詞與會話歷史進行語義脫敏，若引入外部代理極易造成上下文洩漏；
3. **無 SLA 保證之啟發式排程**：僅依賴歷史平均時間或靜態權重分配，無法適應網路抖動與顯存冷啟動。

---

## 2.2 雲邊協同推論與隱私保護技術 (Cloud-Edge Collaborative Inference & Privacy)

為了克服邊緣算力受限與純雲端隱私外洩的二律背反，學界近年積極探索**雲邊協同推論（Cloud-Edge Collaborative Inference）**。

早期的雲邊協同主要採取**神經網路分割（Model Partitioning / Split Computing）**，如 Neurosurgeon 等。此類方法將神經網絡淺層置於邊緣、深層置於雲端，傳輸中間特徵圖（Intermediate Feature Maps）。然而，針對現代自回歸（Autoregressive）LLM，中間隱藏層狀態（Hidden States）維度極大且包含多輪 KV Cache，跨網傳輸的延遲開銷遠超過本地直接推論，且特徵圖容易遭受模型逆向攻擊（Model Inversion Attacks）還原原始輸入。

近年提出的 **PRISM 架構 (Privacy-Aware Routing for Adaptive Cloud–Edge LLM Inference, AAAI 2025)** 代表了一大突破。PRISM 放棄了昂貴的中間特徵傳輸，改為在文字語義層面進行雲邊協同：
* **語義草圖（Semantic Sketch）**：利用雲端強大算力根據脫敏後的輸入生成高階抽象框架；
* **本地重構（Edge Refinement）**：由邊緣設備上的小語言模型（SLM）在完全本地私有環境下，結合原始敏感上下文將草圖具體化。

然而，PRISM 僅探討了「單一邊緣設備對單一公有雲」的情境，在現實個人 AI 叢集中，往往存在多台異質邊緣節點（例如有的節點可跑 7B 模型，有的僅能跑 0.5B 模型）。如何將語義草圖機制擴展至「**多節點異質叢集之動態路由**」，成為本論文核心要解決的開放問題。

---

## 2.3 服務級別協定 (SLA) 與深度強化學習排程 (SLA-Aware Scheduling & DRL)

邊緣運算中的任務排程本質上屬於 NP-hard 的組合最佳化問題。傳統排程演算法包括：
* **均值時間槽啟發式 (Mean-Slot Heuristics, 如 LAS)**：基於長期歷史平均處理能力進行負載均衡，但在突發性（Burstiness）高併發請求下會產生嚴重的佇列累積。
* **靜態加權輪詢 (Weighted Round-Robin)**：無法感知即時顯存置換開銷與網路 RTT 波動。

近年來，基於馬可夫決策過程（Markov Decision Process, MDP）之**深度強化學習（DRL）**被廣泛引入邊緣調度領域。標準的 Deep Q-Network (DQN) 在評估動作價值時容易產生過高估計偏差（Overestimation Bias）。為此，學者提出了 **Dueling Double Deep Q-Network (Dueling DDQN)**：
1. **Double DQN 機制**：將「動作選取」與「目標 Q 值計算」的神經網路參數解耦，大幅降低估計偏差；
2. **Dueling 架構**：將輸出層拆分為**狀態價值函數 $V(s)$** 與**各動作優勢函數 $A(s, a)$**：
   $$Q(s, a; \theta, \alpha, \beta) = V(s; \theta, \beta) + \left( A(s, a; \theta, \alpha) - \frac{1}{|\mathcal{A}|} \sum_{a' \in \mathcal{A}} A(s, a'; \theta, \alpha) \right)$$
   此架構特別適合邊緣排程場景，因為在許多狀態下，系統整體的擁塞程度（由 $V(s)$ 決定）比微調特定動作（由 $A(s, a)$ 決定）具有更關鍵的決定性。

然而，在具有嚴格截止期限（SLA Deadline）的即時系統中，純強化學習的「探索（Exploration）」機制會隨機選取動作，往往導致災難性的 SLA 逾期。本研究借鑒安全強化學習（Safe RL）概念，在 Dueling DDQN 中融合 **SLA 動作遮罩（Action-Masking）**，在決策輸出前先行過濾無效與必超時動作，確保高吞吐下的即時可靠性。

---

## 2.4 系統程式語言 Rust 於低延遲分散式系統之工程優勢

傳統分散式微服務與中介層多採 Go 或 Java 實作，其內建的垃圾回收器（Garbage Collector, GC）在高對象分配率下不可避免地帶來「Stop-the-World」暫停，造成 P99 尾端延遲的不可預測性。而 C/C++ 雖具備極致效能，卻缺乏記憶體安全保證，在多執行緒網路編程中極易發生資料競爭（Data Race）與緩衝區溢位。

**Rust 語言** 結合了 C++ 級別的高效能與微秒級開銷，並具備以下核心特性，使其成為建構 PAC-Route 的最佳選擇：
1. **基於所有權（Ownership）與生命週期（Lifetime）的編譯期記憶體安全**：無 GC 開銷，記憶體在變數離開作用域時即時確定性釋放；
2. **Tokio 高併發非同步非阻塞執行期**：基於事件迴圈與 Work-Stealing 執行緒池，單機可輕鬆維持數萬並發連線；
3. **強大的型別系統與代數資料型別（Enum / Pattern Matching）**：能夠在編譯期保證兩平面隔離邏輯與 SLA 狀態機轉換的正確性，徹底杜絕執行期未定義行為。

