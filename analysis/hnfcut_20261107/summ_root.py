"""root60.jsonl などの表 (問題 × 設定: 根の LP、根のカットの後の下界、閉じたギャップ、状態・時間・ノード、HNFCUT の数え上げ)。
閉じたギャップ = (カットの後 - 根の LP) / (最良の目的値 - 根の LP)。最良の目的値は全ての回の解の最良 (最小化)。"""
import json, re, sys
from collections import defaultdict
recs = [json.loads(l) for f in sys.argv[1:] for l in open(f)]
best = {}
for r in recs:
    o = r["result"].get("objective")
    if o is not None:
        best[r["problem"]] = min(best.get(r["problem"], o), o)
by = defaultdict(dict)
cfgs = []
for r in recs:
    by[r["problem"]][(r["cfg"], r["rep"])] = r
    if r["cfg"] not in cfgs: cfgs.append(r["cfg"])
print("| 問題 | 設定 | 回 | 根の LP | カットの後 | 閉じた | 状態 | 秒 | ノード | 根の数え上げ (勝ち/置き換えだけ/mod-k のカット) |")
print("|---|---|---|---|---|---|---|---|---|---|")
for p, d in by.items():
    for (c, s), r in sorted(d.items(), key=lambda kv: (cfgs.index(kv[0][0]), kv[0][1])):
        i, res = r["info"], r["result"]
        lp, ac = i.get("root_lp"), i.get("root_after_cuts")
        gc = ""
        if lp is not None and ac is not None and p in best and best[p] - lp > 1e-9:
            gc = "%.1f%%" % (100 * (ac - lp) / (best[p] - lp))
        st = ""
        m = re.search(r"cong won (\d+).*only cong (\d+).*cuts (\d+)", i.get("root_stats", ""))
        if m: st = "/".join(m.groups())
        f = lambda v: "-" if v is None else "%.6g" % v
        print(f"| {p} | {c} | {s} | {f(lp)} | {f(ac)} | {gc} | {res.get('status')} | {f(res.get('time'))} | {res.get('nodes')} | {st} |")
