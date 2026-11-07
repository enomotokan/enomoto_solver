"""58 問 (4 スレッド・60 秒、種 1・2) の def 対 cd の集計 (analysis/benchlib.py の report)。使い方: python summ58.py > bench58_summary.txt"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([D / "bench58.jsonl"]))
keys = sorted(k for k in R if "def" in R[k] and "cd" in R[k])
bl.report(R, "def", "cd", keys, noise=bl.load_noise(), label="58 問 種 1・2")
print()
print("問題ごと (時間、未解決は 60): def r1 r2 | cd r1 r2 | cd の選ばれた新しい δ のカット/全体 (r1)")
sep = {}
for l in open(D / "bench58.jsonl"):
    r = json.loads(l)
    if r["cfg"] == "cd" and r.get("rep") == 1 and r.get("sep"):
        sep[r["problem"]] = r["sep"]
probs = sorted({k[0] for k in keys})
for p in probs:
    ts = {c: [bl.t(R[(p, rep)][c]) if (p, rep) in R and c in R[(p, rep)] else float("nan") for rep in (1, 2)] for c in ("def", "cd")}
    s = sep.get(p, {})
    print(f"{p:28s} {ts['def'][0]:6.1f} {ts['def'][1]:6.1f} | {ts['cd'][0]:6.1f} {ts['cd'][1]:6.1f} | {int(s.get('chosen_new') or 0)}/{int(s.get('chosen_all') or 0)}")
