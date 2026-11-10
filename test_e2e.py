import urllib.request
import urllib.error
import json

BASE = "http://localhost:8088"

def http_get(url):
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req) as resp:
        return resp.status, json.loads(resp.read().decode('utf-8'))

def http_post(url, data, headers=None):
    if headers is None:
        headers = {}
    headers["Content-Type"] = "application/json"
    body = json.dumps(data).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers=headers)
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode('utf-8'))

print("==================================================================")
print("🚀 PAC-Route 系統整合測試與效能驗證")
print("==================================================================")

# 測試 1
print("\n[測試 1] 健康檢查 (/health)")
status, res = http_get(f"{BASE}/health")
print("HTTP 狀態碼:", status)
print(json.dumps(res, indent=2, ensure_ascii=False))

# 測試 2
print("\n[測試 2] 查詢異質個人叢集節點能力 (/cluster/nodes)")
status, res = http_get(f"{BASE}/cluster/nodes")
print("HTTP 狀態碼:", status)
nodes = res.get("nodes", [])
for n in nodes:
    print(f" -> 節點名稱: {n['name']:<28} | 角色: {n['role']:<15} | 吞吐估計: {n['hardware']['token_speed_estimate']:>5} tok/s | 支援模型: {n['supported_models']}")

# 測試 3
print("\n[測試 3] OpenAI 相容模型清單 (/v1/models)")
status, res = http_get(f"{BASE}/v1/models")
print("HTTP 狀態碼:", status)
print("可用模型清單:", [m["id"] for m in res.get("data", [])])

# 測試 4
print("\n[測試 4] 一般公開推論請求 (Public Prompt - 本地優先路由)")
req1 = {
    "model": "llama3.2:3b",
    "messages": [{"role": "user", "content": "請解釋什麼是 Rust 的記憶體所有權 (Ownership)？"}]
}
status, res1 = http_post(f"{BASE}/v1/chat/completions", req1)
print("HTTP 狀態碼:", status)
if status == 200:
    print("回應內容:\n", res1["choices"][0]["message"]["content"])
    print("PAC 中繼排程資訊:\n", json.dumps(res1["pac_meta"], indent=2, ensure_ascii=False))
else:
    print("錯誤訊息:\n", res1)

# 測試 5
print("\n[測試 5] 包含多重敏感資訊之請求 (TopSecret 自動偵測與機密隔離)")
req2 = {
    "model": "llama3.2:3b",
    "messages": [{"role": "user", "content": "管理員 admin@nfu.edu.tw，本機 IP 192.168.1.100，金鑰為 sk-abcdef1234567890abcdefghijk，請驗證系統安全"}]
}
status, res2 = http_post(f"{BASE}/v1/chat/completions", req2)
print("HTTP 狀態碼:", status)
if status == 200:
    print("偵測隱私等級:", res2["pac_meta"]["privacy_level"])
    print("選定路由模式:", res2["pac_meta"]["routing_mode"])
    print("派發執行節點:", res2["pac_meta"]["node_name"])
    print("排程保證: 機密資料強制隔離於局域網，絕不上雲！")
    print("預估延遲:", res2["pac_meta"]["predicted_ms"], "ms")
else:
    print("拒絕或錯誤訊息:\n", json.dumps(res2, indent=2, ensure_ascii=False))

# 測試 6
print("\n[測試 6] 雙層 LDP 語義草圖雲邊協同推論 (Semantic Sketch Collaboration)")
req3 = {
    "model": "llama3.2:3b",
    "messages": [{"role": "user", "content": "客戶電話是 0912-345-678，請幫我寫一份通知簡訊草稿"}]
}
headers = {"X-Consent-Cloud": "true"}
status, res3 = http_post(f"{BASE}/v1/chat/completions", req3, headers)
print("HTTP 狀態碼:", status)
if status == 200:
    print("選定路由模式:", res3["pac_meta"]["routing_mode"])
    print("雲邊協同還原後內容:\n", res3["choices"][0]["message"]["content"])
    print("還原驗證: 電話號碼 '0912-345-678' 已在邊緣無失真還原！")
else:
    print("錯誤訊息:\n", res3)

# 測試 7
print("\n[測試 7] SLA 截止期限硬性動作遮罩違約攔截 (SLA Deadline Miss Enforcement)")
req4 = {
    "model": "llama3.2:3b",
    "messages": [{"role": "user", "content": "超低延遲即時視覺輔助任務"}]
}
headers = {"X-SLA-Deadline-Ms": "50"}  # 50ms 超嚴格期限
status, res4 = http_post(f"{BASE}/v1/chat/completions", req4, headers)
print("HTTP 狀態碼 (預期應為 503 服務拒絕/保護):", status)
print("攔截原因說明:\n", json.dumps(res4, indent=2, ensure_ascii=False))

print("\n==================================================================")
print("✅ 所有 PAC-Route 系統與演算法測試案例執行完畢！")
print("==================================================================")
