"""res.jsonl の集計: python analysis/confcut_20261124/summ.py [res.jsonl]
問題ごと・設定ごとに、種で平均した時間 (時間切れは 120 秒)・解けた回・ノード・ノードあたりの時間 (ms)・LP に入った衝突/証明のカット・
それが入ったノードのうち刈った割合を出し、最後に設定ごとの時間の SGM (ずらし 10 秒) と既定との対の比を出す。"""
import json, math, sys, collections
from pathlib import Path
F = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "res.jsonl"
R = collections.defaultdict(lambda: collections.defaultdict(dict))
for l in F.read_text().splitlines():
    r = json.loads(l); R[r["problem"]][r["cfg"]][r["seed"]] = r
CF = [c for c in ["def", "c", "p", "cp", "cn", "t4"] if any(c in d for d in R.values())]
def t(res, tl): return res["time"] if res.get("status") == "optimal" else tl
print(f"| 問題 | 設定 | 解けた | 時間 | ノード | ms/ノード | LP 反復/ノード | 入った衝突 | 入った証明 | 入ったノード (刈った) | 衝突の候補 |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
sg = collections.defaultdict(list)
for p, d in R.items():
    for c in CF:
        if c not in d: continue
        rs = list(d[c].values()); tl = rs[0]["tl"]
        ts = [t(r["result"], tl) for r in rs]; nd = [r["result"]["nodes"] or 0 for r in rs]
        dg = [r["result"].get("diag", {}) for r in rs]
        m = lambda k: sum(x.get(k, 0) for x in dg) / len(dg)
        ok = sum(r["result"].get("status") == "optimal" for r in rs)
        tn = sum(ts) / len(ts); nn = sum(nd) / len(nd)
        print(f"| {p} | {c} | {ok}/{len(rs)} | {tn:.1f} | {nn:.0f} | {1000 * tn / max(nn, 1):.2f} | {m('lp_iters') / max(nn, 1):.1f} | {m('in_lp_conf'):.0f} | {m('in_lp_proof'):.0f} | {m('nodes_with'):.0f} ({m('pruned_with'):.0f}) | {m('cand_conf'):.0f} |")
        for s, r in d[c].items(): sg[c].append((p, s, t(r["result"], tl)))
print()
base = {(p, s): v for p, s, v in sg["def"]}
for c in CF:
    v = sg[c]; g = math.exp(sum(math.log(x + 10) for _, _, x in v) / len(v)) - 10
    pr = [math.log((x + 10) / (base[(p, s)] + 10)) for p, s, x in v if (p, s) in base]
    print(f"{c}: 回 {len(v)} SGM {g:.2f} 秒、既定との対の比 {math.exp(sum(pr) / max(len(pr), 1)):.3f} (解けた {sum(x < 119.9 for _, _, x in v)})")
