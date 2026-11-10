# 第一章 緒論 (Introduction)

## 1.1 研究背景

在過去數年間，以 Transformer 架構為基礎之大型語言模型（Large Language Models, LLMs，如 Llama、Qwen、DeepSeek 系列）以及前沿多代理系統（Multi-Agent Systems）取得了突破性的進展。這些模型不再僅僅被動回答使用者提問，而是扮演能夠自主解構複雜目標、編寫並執行程式碼、協同除錯、呼叫外部工具（Tool/Function Calling）的智慧自主代理（Autonomous Agents）。

然而，隨著多代理系統在工程實踐中的普及，傳統單純仰賴公有雲 API（如 OpenAI、Anthropic）或單一本地工作站的運算範式，面臨了前所未有的雙重嚴峻挑戰：

1. **頻寬與隱私風險**：
   在現代軟體工程與個人輔助場景中，代理工作流頻繁存取個人工作區檔案、帳號密碼憑證、資料庫連接字串、通訊軟體私密歷史對話乃至敏感商業邏輯。若將包含多輪歷史與工具呼叫追蹤（Tool Traces）之完整上下文（Context Window）無差別上傳至集中式公有雲，極易造成重大隱私洩漏與資安法規（如 GDPR、個人資料保護法）之違規風險。
   
2. **單機算力瓶頸與請求排隊堵塞**：
   若退而求其次，將所有推論請求全數收攏至個人本地設備，常見的高階消費級顯示卡（如單張 NVIDIA GeForce RTX 3060/4090）又往往受制於有限的顯存容量（VRAM）與並行運算通道。在一個典型的多代理協同場景中，主代理（Lead Agent）往往會在短時間內並行拆分出數個乃至十數個子代理（Sub-agents），同步進行程式碼分析、文獻檢索與單元測試。若所有高頻發出的推論請求均湧向單一 GPU，將引發劇烈的顯存爭用、頻繁的模型載入與上下文換出（Context Swapping），造成長尾延遲暴增與請求排隊互相阻塞。

為了化解此一矛盾，使用者與邊緣實驗室逐漸傾向於利用區域網路（Local Area Network, LAN）中既有的閒置設備，組建低成本、私有化的「**異質個人 AI 叢集（Heterogeneous Personal AI Cluster）**」。此類叢集通常由性能各異的硬體混編而成：例如一台配備高階顯卡（如 RTX 4090 24GB）的主工作站、一台日常輕量開發 PC（如 RTX 3060 12GB）、數台搭載 Linux 的 ARM 單板電腦（如 Raspberry Pi 5 8GB），乃至負責特定感測任務之微控制器（如 ESP32）。

---

## 1.2 研究動機與現存技術痛點

儘管異質個人 AI 叢集在硬體持有成本與理論算力聚合上具備極高潛力，然而在實際落實為高可靠度、低延遲且具備強隱私保證之推論系統時，現有文獻與業界開源架構暴露出三大關鍵缺陷：

### 痛點一：粗粒度二元路由與會話歷史隱私洩漏 (Context Leakage in Binary Routing)
現有針對雲邊協同 LLM 推論之研究，多半停留在簡單的**二元路由（Binary Routing）**思維——即單純在接收到提示詞（Prompt）時，透過輕量分類器判斷該提示詞屬於「留存本地」或「直傳雲端」。
此種粗粒度策略存在嚴重漏洞：
* **長會話隱私盲區**：在多輪對話與自主代理執行過程中，最新的提示詞可能表面上不具備敏感特徵，但隨附之系統訊息（System Prompt）、歷史上下文或工具回傳紀錄中卻常包含真實 IP、電子郵件、私鑰或未公開源碼。單純判定單一提示詞，無法阻止歷史上下文的隱私外洩。
* **二元決策的極端妥協**：若將任何稍具敏感度的任務完全封鎖於本機，則低算力之邊緣單板電腦（SBC）極易發生過載當機；反之，若為了追求生成速度而將任務拋向雲端，則使用者隱私蕩然無存。

### 痛點二：異質節點 SLA 動態排程機制之缺失 (Lack of Dynamic SLA Adaptability)
在包含工作站、筆記型電腦與樹莓派之異質叢集中，節點的可用顯存、計算吞吐量（Tokens per Second）以及局域網無線傳輸條件均具有高度動態波動性。
現有系統（包括傳統邊緣運算卸載之均值槽排程法，以及 NVIDIA 最新發布之 Personal AI Router, PAIR）往往採用靜態輪詢（Round-Robin）或基於歷史平均執行時間之啟發式演算法。此類方法忽略了：
1. **模型冷啟動與顯存置換開銷（Model Loading Latency）**：若請求所需之模型未事先常駐於該節點顯存中，從磁碟載入數 GB 之權重檔動輒產生數千毫秒之延遲，遠高於推論本身；
2. **佇列積壓與首字延遲（TTFT / Queueing Delay）**；
3. **服務級別協定（SLA）截止期限約束**：對於即時電腦視覺追蹤（如 YOLOv8）或對話式程式輔助，任務具有嚴苛的硬性截止期限（Deadline）。傳統排程器無法在決策階段前瞻性排除必將逾期的節點，導致大量任務違約（Deadline Miss）。

### 痛點三：中介轉發軟體之執行期開銷與 GC 尾端抖動 (Runtime Tail-Latency Spikes)
現有個人 AI 路由器多以直譯語言（如 Python）配合簡易 HTTP Server 實作。在高併發、多代理持續送出微小封包與串流推論請求時，全域直譯器鎖（GIL）之執行緒競爭與垃圾回收機制（Garbage Collection, GC）會週期性引發數十至數百毫秒的延遲尖峰（Tail Latency Spike），進而破壞微秒級之路由開銷與端到端延遲的可預測性。在嚴苛的即時排程環境中，轉發層本身必須具備極致的效能、記憶體安全與零 GC 停頓特性。

---

## 1.3 研究目標與核心貢獻

針對前述痛點，本研究以去中心化代理隔離機制與相容端點為基石，設計並採用系統程式語言 **Rust** 實作了一套兼具隱私保護與 SLA 保證的高效能路由調度系統——**PAC-Route (Privacy-Constrained Cloud-Edge Cluster Routing for Heterogeneous AI Clusters)**。

本研究之主要貢獻包括：

1. **提出去中心化兩平面隔離架構 (Uncentralized Two-Plane Architecture)**：
   在系統層級實體隔離「隱私授權外部平面（Consent-Gated Plane）」與「本地推理平面（Local Inference Plane）」。本系統落實零信任（Zero-Trust）原則，強制規定任何資料或會話記錄跨出本機網段前，必須通過密碼學授權或明確之使用者授權閘門，達成「絕不靜默外洩（Never Silently Leak）」的強保證。

2. **研發結合動作遮罩之 Dueling DDQN SLA 排程演算法 (SLA Action-Masked DRL Scheduler)**：
   形式化建模端到端延遲分解公式（包含傳輸、排隊、顯存載入與執行耗時）。導入深度強化學習之 Dueling Double Deep Q-Network (Dueling DDQN) 演算法，並創新性地引入 **SLA 動作遮罩機制（Action-Masking）**。在探索與利用過程中，系統會於動作生成階段直接動態剔除預估將超出 SLA 期限之候選節點，有效避免探索期的高昂違約代價。

3. **建構動態軟體閘門與雙層 LDP 語義草圖協同管線 (2-Layer LDP Semantic Sketch Pipeline)**：
   捨棄傳統二元路由，提出包含「完全本機推論」、「直接雲端推論」與「雲邊語義草圖協同」之三模式執行管線。在協同模式中，邊緣端首先對輸入進行敏感實體（PII/Secrets）提取，施加雙層本地差分隱私（2-Layer LDP）數值與類別擾動；雲端強大 LLM 僅基於去敏抽象提示詞生成宏觀語義草圖（Semantic Sketch）；最後交由本地邊緣輕量模型（SLM）填補私有實體細節，徹底根絕歷史會話與上下文外洩。

4. **基於 Rust 打造純原生、相容主流生態之高效能系統實作**：
   採用 Rust 與 Tokio 非同步非阻塞執行期實作 PAC-Route 閘道器，並原生整合基於 UDP 廣播之 mDNS/DNS-SD 局域網零配置節點能力發現機制。系統全面向下相容 OpenAI API (`/v1/chat/completions`) 與 Ollama 原生端點，使得既有之 AI 程式開發工具（如 Cursor、VSCode Continue、Open-WebUI 等）無需修改任何前端程式碼即可無縫接入。

---

## 1.4 論文架構組織

本論文共分為六章，整體章節組織安排如下：
* **第一章 緒論 (Introduction)**：闡述研究背景、動機、現有技術痛點、核心研究目標與主要學術貢獻。
* **第二章 文獻探討與相關技術 (Related Work)**：全面回顧異質個人 AI 叢集路由、雲邊協同推論隱私保護機制（包括 PRISM 與差分隱私技術）、強化學習在邊緣運算 SLA 調度中之應用，以及 Rust 語言於低延遲分散式系統之工程優勢。
* **第三章 PAC-Route 系統架構與數學建模 (System Design & Formal Modeling)**：詳述去中心化兩平面設計原理、mDNS 節點能力宣告協定、端到端延遲之形式化拆解模型、SLA 動作遮罩數學表徵與雙層差分隱私理論證明。
* **第四章 演算法設計與 Rust 核心系統實作 (Implementation Details)**：展示三模式軟體閘門演算法、語義草圖產生與本地還原流程、Dueling DDQN 強化學習狀態動作轉移邏輯，以及 Rust 非同步網路代理模組架構。
* **第五章 實驗評估與效能分析 (Experimental Evaluation)**：於實體混合硬體測試集上進行首字延遲（TTFT）、每字元延遲（TPOT）、P95 尾端延遲、SLA 違約率對比，並實施消融實驗（Ablation Study）與網路斷線隱私安全驗證。
* **第六章 結論與未來研究方向 (Conclusion & Future Work)**：總結本研究成果與學術實務價值，並提出結合分散式 LoRA 微調與硬體安全隔離（TEE）之未來展望。

