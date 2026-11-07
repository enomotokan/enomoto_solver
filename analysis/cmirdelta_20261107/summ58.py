"""58 問 (4 スレッド・60 秒、種 1・2) の def 対 cd の集計 (analysis/benchlib.py の report)。使い方: python summ58.py > bench58_summary.txt
2 回目 (同じ wheel の def 対 cdn、bench58n.jsonl): python summ58.py n > bench58n_summary.txt"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
N = sys.argv[1:] == ["n"]
F = D / ("bench58n.jsonl" if N else "bench58.jsonl")
B = "cdn" if N else "cd"
R = bl.table(bl.load([F]))
keys = sorted(k for k in R if "def" in R[k] and B in R[k])
bl.report(R, "def", B, keys, noise=bl.load_noise(), label="58 問 種 1・2")
print()
print("問題ごと (時間、未解決は 60): def r1 r2 | cd r1 r2 | cd の選ばれた新しい δ のカット/全体 (r1)")
sep = {}
for l in open(F):
    r = json.loads(l)
    if r["cfg"] == B and r.get("rep") == 1 and r.get("sep"):
        sep[r["problem"]] = r["sep"]
probs = sorted({k[0] for k in keys})
for p in probs:
    ts = {c: [bl.t(R[(p, rep)][c]) if (p, rep) in R and c in R[(p, rep)] else float("nan") for rep in (1, 2)] for c in ("def", B)}
    s = sep.get(p, {})
    print(f"{p:28s} {ts['def'][0]:6.1f} {ts['def'][1]:6.1f} | {ts[B][0]:6.1f} {ts[B][1]:6.1f} | {int(s.get('chosen_new') or 0)}/{int(s.get('chosen_all') or 0)}")
