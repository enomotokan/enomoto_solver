"""1 分担の報告: python shardrep.py res_M_sK.jsonl [基準 試作] (既定 def t3)。件数・crash・解けた数・目的値の食い違い・1.25 倍超の遅速・時間切れの暫定解の差を出す"""
import json, sys
f = sys.argv[1]; base, proto = (sys.argv[2], sys.argv[3]) if len(sys.argv) > 3 else ("def", "t3")
R = {}
for l in open(f):
    if l.strip():
        r = json.loads(l); R[(r["problem"], r["seed"], r["cfg"])] = r
print(f"件数 {len(R)}")
crash = [k for k, r in R.items() if r["result"].get("status") not in ("optimal", "time_limit", "infeasible")]
print("crash・その他の状態:", [(k, R[k]["result"].get("status")) for k in crash] or "なし")
for c in (base, proto):
    rs = [r for k, r in R.items() if k[2] == c]
    print(f"{c}: {len(rs)} 回、解けた {sum(r['result'].get('status') == 'optimal' for r in rs)}")
pairs = sorted({(k[0], k[1]) for k in R if (k[0], k[1], base) in R and (k[0], k[1], proto) in R})
mism, slow, fast, tl = [], [], [], []
for p, s in pairs:
    a, b = R[(p, s, base)]["result"], R[(p, s, proto)]["result"]
    if a.get("status") == "optimal" and b.get("status") == "optimal":
        za, zb = a["objective"], b["objective"]
        if abs(za - zb) > 1e-6 * max(1.0, abs(za)): mism.append((p, s, za, zb))
    if a.get("status") != b.get("status") and "optimal" in (a.get("status"), b.get("status")):
        if a.get("status") == "infeasible" or b.get("status") == "infeasible": mism.append((p, s, a.get("status"), b.get("status")))
    ta = a["time"] if a.get("status") == "optimal" else R[(p, s, base)]["tl"]
    tb = b["time"] if b.get("status") == "optimal" else R[(p, s, proto)]["tl"]
    if a.get("status") == "optimal" or b.get("status") == "optimal":
        if tb > 1.25 * ta and tb - ta > 1: slow.append((p, s, round(ta, 1), round(tb, 1)))
        if ta > 1.25 * tb and ta - tb > 1: fast.append((p, s, round(ta, 1), round(tb, 1)))
    if a.get("status") == "time_limit" and b.get("status") == "time_limit":
        tl.append((p, s, a.get("objective"), b.get("objective"), a.get("bound"), b.get("bound")))
print("対の数", len(pairs))
print("目的値の食い違い:", mism or "なし")
print(f"{proto} が 1.25 倍超遅い:", slow or "なし")
print(f"{proto} が 1.25 倍超速い:", fast or "なし")
print("両方時間切れ (暫定解 基準/試作, 下界 基準/試作):")
for t in tl: print("  ", t)
