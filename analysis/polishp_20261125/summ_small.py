"""POLISH_WARM=d の小さな確かめ (res_small.jsonl) の集計: 既定 def_t1 との比 (求解の時間 + 1 秒の幾何平均)、解けた数、遅れ、
既知の値との食い違い (最適と言った行が solu.json の値から相対 1e-6 を超えてずれる)、上限の超過"""
import json, math
sol = json.load(open("solu.json"))
rs = [json.loads(l) for l in open("res_small.jsonl")]
d = {(r["problem"], r["cfg"]): r["result"] for r in rs}
P = list(dict.fromkeys(r["problem"] for r in rs))
print(len(rs), "行、crash・取得失敗", sum(r["result"].get("status") in ("crash", "fetch_error") for r in rs))
t = lambda v: min(v.get("time", 60), 60) if v.get("status") == "optimal" else 60
for c in ("s1wd_t1", "s2wd_t1"):
    lr, slow, solved, mism, over = [], [], [0, 0], [], []
    for p in P:
        a, b = d[(p, "def_t1")], d[(p, c)]
        solved[0] += a["status"] == "optimal"; solved[1] += b["status"] == "optimal"
        lr.append(math.log((t(b) + 1) / (t(a) + 1)))
        if (t(b) + 1) / (t(a) + 1) > 1.25:
            slow.append((p, round(t(a), 2), round(t(b), 2)))
        k = sol.get(p, {}).get("value")
        if b["status"] == "optimal" and k is not None and abs(b["objective"] - k) > 1e-6 * max(1, abs(k)):
            mism.append((p, b["objective"], k))
        if b.get("time", 0) > 60.5:
            over.append((p, round(b["time"], 2)))
    print(f"{c}: 比 {math.exp(sum(lr) / len(lr)):.3f}、解けた数 {solved[0]} → {solved[1]}、1.25 倍超の遅れ {slow}、食い違い {mism}、60.5 秒超 {over}")
print("既定の 60.5 秒超", [(p, round(d[(p, 'def_t1')]["time"], 2)) for p in P if d[(p, "def_t1")].get("time", 0) > 60.5])
