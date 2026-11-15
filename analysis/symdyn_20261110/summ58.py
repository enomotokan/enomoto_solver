"""60 秒の比べの集計 (benchlib.report): python summ58.py [res58.jsonl] [設定,設定] [問題を絞る: all|sym]
sym なら symscan.jsonl で生成元のある問題だけ。"""
import json, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import benchlib as bl  # noqa: E402
rows = bl.load([HERE / (sys.argv[1] if len(sys.argv) > 1 else "res58.jsonl")])
R = bl.table(rows)
which = sys.argv[3] if len(sys.argv) > 3 else "all"
scan = {json.loads(l)["problem"]: json.loads(l) for l in open(HERE / "symscan.jsonl")}
sym = {p for p, r in scan.items() if int(r["gens"] or 0) > 0 or int(r["refl"] or 0) > 0}
noise = bl.load_noise()
for B in (sys.argv[2].split(",") if len(sys.argv) > 2 else ["dyn", "refl"]):
    keys = sorted(k for k in R if "def" in R[k] and B in R[k] and (which == "all" or k[0] in sym))
    bl.report(R, "def", B, keys, noise=noise, label=f"def vs {B} ({which}, {len(keys)} 回)")
    print()
