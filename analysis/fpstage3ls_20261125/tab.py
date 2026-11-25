"""res_one.jsonl の表 (1 スレッド)。問題ごとに札ごとの 3 種の秒 (時間切れは上限) と目的値、組ごとの幾何平均 (shift 10)。"""
import json, math, sys
from collections import defaultdict
D = __file__.rsplit("/", 1)[0]
TL = {"10teams": 120, "neos-1171448": 120, "CMS750_4": 200, "noswot": 60, "pg": 60, "swath3": 120, "neos-1582420": 120, "app1-1": 120}
SETS = {"効いた": ["10teams", "neos-1171448", "CMS750_4", "noswot", "pg"],
        "害 (heuradapt)": ["eil33-2", "swath1", "mas76", "markshare_4_0", "blend2", "misc07", "pk1", "roll3000"],
        "害 (combo)": ["swath3", "neos-1582420", "app1-1"]}
tags = sys.argv[1:] or ["adm", "lsa", "lsb"]
R = defaultdict(dict)
for l in open(f"{D}/res_one.jsonl"):
    r = json.loads(l)
    if r["result"] is None: continue
    R[(r["problem"], r["tag"])][r["seed"]] = r["result"]
def t(p, res):
    tl = TL.get(p, 60)
    return min(res["time"], tl) if res["status"] in ("optimal", "infeasible") else tl
for name, ps in SETS.items():
    print(f"\n### {name}\n\n| 問題 | " + " | ".join(tags) + " |\n|---|" + "---|" * len(tags))
    sg = {k: [] for k in tags}; ns = {k: 0 for k in tags}
    for p in ps:
        cells = []
        for k in tags:
            d = R.get((p, k), {})
            xs = []
            for s in (1, 2, 3):
                if s not in d: xs.append("-"); continue
                v = t(p, d[s]); sg[k].append(v); ns[k] += d[s]["status"] == "optimal"
                xs.append(f"{v:.0f}" if d[s]["status"] == "optimal" else f"tl({d[s]['objective']:.6g})" if d[s]["objective"] is not None else "tl(—)")
            cells.append(" ".join(xs))
        print(f"| {p} | " + " | ".join(cells) + " |")
    print("| 幾何平均 (解けた数) | " + " | ".join(f"{math.exp(sum(math.log(v+10) for v in sg[k])/max(len(sg[k]),1))-10:.1f} ({ns[k]})" for k in tags) + " |")
