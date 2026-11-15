"""58 問 (4 スレッド・60 秒) の def・cd を種 1・2 で回す待ち行列 (続きから回せる)。使い方: python queue58.py  (bench58.jsonl に足す)"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
OUT = "bench58.jsonl"
CFGS = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def", "cd"]
def done():
    s = set()
    if (D / OUT).exists():
        for l in open(D / OUT):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"], r["rep"]))
            except Exception:
                pass
    return s
for rep in [1, 2]:
    for p in PROBS:
        for cfg in CFGS:
            if (p, cfg, rep) in done():
                continue
            try:
                subprocess.run([sys.executable, str(D / "run.py"), cfg, p, "60", str(rep), OUT], timeout=60 + 600)
            except subprocess.TimeoutExpired:
                with open(D / OUT, "a") as f:
                    f.write(json.dumps({"problem": p, "cfg": cfg, "tl": 60, "rep": rep, "result": {"status": "killed"}}) + "\n")
