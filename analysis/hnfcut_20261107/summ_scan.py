"""scan.jsonl (collection の探索) の集計: 数・飛ばした理由・合同式の出た問題の表 (根の LP 点の内訳つき)。"""
import json, sys
from collections import Counter
recs = [json.loads(l) for l in open(sys.argv[1] if len(sys.argv) > 1 else "scan.jsonl")]
c = Counter()
hits = []
for r in recs:
    if r.get("skip"): c["skip_" + r["skip"]] += 1; continue
    if r.get("err"): c["err"] += 1; continue
    h = r.get("hnf")
    if not h: c["no_hnf_line (前処理で何も減らない・前処理で解けた等)"] += 1; continue
    c["analyzed"] += 1
    if h.get("int_eq"): c["int_eq>0"] += 1
    if h.get("infeasible"): c["infeasible"] += 1
    if h.get("fixes"): c["fixes"] += 1
    if h.get("congs"): c["congs"] += 1; hits.append(r)
print("問題", len(recs), dict(c))
print("| 問題 | 整数の等式 | 合同式 (一般整数, 詰まる, 最大の法) | 最終の表 | 根の LP 点: 列 (分数 / 整数で合わない / 合う (上下限)) | 20 秒の結果 |")
print("|---|---|---|---|---|---|")
for r in sorted(hits, key=lambda r: -((r.get("lp") or {}).get("frac", 0) + (r.get("lp") or {}).get("off", 0))):
    h = r["hnf"]; lp = r.get("lp")
    lps = f"{lp['cols']} ({lp['frac']} / {lp['off']} / {lp['on']} ({lp['atb']}))" if lp else "未到達"
    res = r.get("result") or {}
    print(f"| {r['problem']} | {h['int_eq']} | {h['congs']} ({h['gen']}, {h['tighten']}, {h['max_mod']}) | {r.get('final_congs', 0)} | {lps} | {res.get('status')} {res.get('objective')} / {res.get('bound')} |")
