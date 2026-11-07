"""58 問 (4 スレッド・60 秒、種 1・2) の def 対 h の集計 (analysis/benchlib.py の report) と、h で切り替えが起きた問題。
使い方: python summ58.py [jsonl] > bench58_summary.txt"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
F = Path(sys.argv[1]) if len(sys.argv) > 1 else D / "bench58.jsonl"
R = bl.table(bl.load([F]))
keys = sorted(k for k in R if "def" in R[k] and "h" in R[k])
bl.report(R, "def", "h", keys, noise=bl.load_noise(), label="58 問 種 1・2")
print()
print("h で双子に切り替えた問題 (種、最初の切り替えの行) と時間 (def → h、未解決は 60):")
for l in open(F):
    r = json.loads(l)
    if r["cfg"] == "h" and any("heavy" in n for n in r["notes"]):
        k = (r["problem"], r["rep"])
        td = bl.t(R[k]["def"]) if "def" in R.get(k, {}) else float("nan")
        print(f"  {r['problem']:24s} 種 {r['rep']} {td:6.1f} → {bl.t(r['result']):6.1f}  {[n[:110] for n in r['notes'] if 'heavy' in n][0]}")
