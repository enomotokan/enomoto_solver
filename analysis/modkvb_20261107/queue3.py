"""queue2.py の後: 候補の数の上限つき mk50 (ENOMOTO_T_MIP_MODKVB_MAXCANDS=50) を 58 問の 60 秒 (種 1・2、b58.jsonl) と根 (種 1・2、root58.jsonl) で。
使い方: python queue3.py (続きから回せる)"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
def done(out):
    s = set()
    if (D / out).exists():
        for l in open(D / out):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"], r.get("rep", 1)))
            except Exception:
                pass
    return s
def run(p, cfg, tl, rep, out, root=False):
    if (p, cfg, rep) in done(out):
        return
    cmd = [sys.executable, str(D / "run.py"), cfg, p, str(tl), str(rep), out] + (["root"] if root else [])
    try:
        subprocess.run(cmd, timeout=tl + 900)
    except subprocess.TimeoutExpired:
        pass
for rep in [1, 2]:
    for p in PROBS:
        run(p, "mk50", 60, rep, "b58.jsonl")
for rep in [1, 2]:
    for p in PROBS:
        run(p, "mk50", 60, rep, "root58.jsonl", True)
