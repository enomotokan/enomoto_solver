"""根の比べの集計 (root58.jsonl、HiGHS は highs_root58.jsonl)。使い方: python summ_root.py [設定 ...] > root58_summary.txt
問題ごとに、根の下界と最良の既知値 (bench58.jsonl と root58.jsonl の optimal の値、無ければ最良の暫定解) の相対のずれ、根の時間、
新しい δ の集計 (勝った回・選ばれたカット/選ばれた全カット) を並べる。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
args = sys.argv[1:]
# 2 回目: python summ_root.py n  (root58n.jsonl の def・cdn、HiGHS と最良の既知値は 1 回目のファイルも使う)
second = args[:1] == ["n"]
cfgs = ["def", "cdn"] if second else (args or ["def", "cd", "cdg"])
rows = []
for f in (["root58n.jsonl", "bench58n.jsonl"] if second else []) + ["root58.jsonl", "highs_root58.jsonl", "bench58.jsonl"]:
    if (D / f).exists():
        for l in open(D / f):
            try:
                rows.append(json.loads(l))
            except Exception:
                pass
root = defaultdict(dict)
R = defaultdict(dict)
for r in rows:
    R[(r["problem"], (r.get("rep", 1), r.get("root", False), r["cfg"]))][r["cfg"]] = r["result"]
    if r.get("root") and r["cfg"] in cfgs + ["highs"] and not (second and r["cfg"] == "def" and r["problem"] in root and "def" in root[r["problem"]]):
        root[r["problem"]][r["cfg"]] = r
ref = bl.best_known(R)
def gap(r):
    res = r["result"]
    b = res.get("root_bound", res.get("bound")) if r["cfg"] != "highs" else res.get("bound")
    return bl.rel_gap(b, ref.get(r["problem"]))
print(f"{'問題':28s} " + " ".join(f"{c+'_gap':>10s} {c+'_t':>7s}" for c in cfgs + ["highs"]) + f"  {cfgs[1]}: wins chosen_new/all")
agg = defaultdict(list)
for p in sorted(root):
    d = root[p]
    line = f"{p:28s} "
    for c in cfgs + ["highs"]:
        if c in d:
            g = gap(d[c]); t = d[c]["result"].get("time")
            line += f" {g if g is not None else float('nan'):10.3e} {t if t is not None else float('nan'):7.2f}"
        else:
            line += f" {'-':>10s} {'-':>7s}"
    s = d.get(cfgs[1], {}).get("sep") or {}
    if s:
        line += f"  {int(s.get('wins') or 0)} {int(s.get('chosen_new') or 0)}/{int(s.get('chosen_all') or 0)}"
    print(line)
    if all(c in d for c in cfgs):
        for c in cfgs[1:]:
            ga, gb = gap(d[cfgs[0]]), gap(d[c])
            if ga is not None and gb is not None:
                agg[c + "_gap"].append(math.log10(max(gb, 1e-6)) - math.log10(max(ga, 1e-6)))
            ta, tb = d[cfgs[0]]["result"].get("time"), d[c]["result"].get("time")
            if ta is not None and tb is not None:
                agg[c + "_t"].append(math.log((tb + 1) / (ta + 1)))
print()
for k, v in agg.items():
    nz = [x for x in v if abs(x) > 1e-3]
    print(f"{k} vs {cfgs[0]}: 問題 {len(v)}、平均 {sum(v)/len(v):+.4f} (log10 のずれ / log((t+1)) の比)、良い {sum(x < -1e-3 for x in v)} 悪い {sum(x > 1e-3 for x in v)}"
          + (f" 時間の比 {math.exp(sum(v)/len(v)):.3f}" if k.endswith('_t') else ""))
