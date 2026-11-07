"""絞った形 (cdn、ENOMOTO_MIP_CMIRDELTA=n) の計測の待ち行列 (2026-11-07 の 2 回目、続きから回せる)。def も同じ wheel で回し直す。
順に (1) 効く問題 8 問 × 種 1〜4 (effn.jsonl)、(2) 58 問の根 (root58n.jsonl、種 1)、(3) 58 問 × 種 1・2 (bench58n.jsonl)。4 スレッド・60 秒"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
EFF = ["neos-3083819-nubu", "neos-911970", "hypothyroid-k1", "p200x1188c", "sp150x300d", "h80x6320d", "qnet1", "drayage-25-23"]
CFGS = ["def", "cdn"]
def done(out):
    s = set()
    if (D / out).exists():
        for l in open(D / out):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"], r.get("rep")))
            except Exception:
                pass
    return s
def run(p, cfg, rep, out, root=False):
    if (p, cfg, rep) in done(out):
        return
    cmd = [sys.executable, str(D / "run.py"), cfg, p, "60", str(rep), out] + (["root"] if root else [])
    try:
        subprocess.run(cmd, timeout=60 + 600)
    except subprocess.TimeoutExpired:
        with open(D / out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "tl": 60, "rep": rep, "root": root, "result": {"status": "killed"}}) + "\n")
for rep in [1, 2, 3, 4]:
    for p in EFF:
        for c in CFGS:
            run(p, c, rep, "effn.jsonl")
for p in PROBS:
    for c in CFGS:
        run(p, c, 1, "root58n.jsonl", root=True)
for rep in [1, 2]:
    for p in PROBS:
        for c in CFGS:
            run(p, c, rep, "bench58n.jsonl")
