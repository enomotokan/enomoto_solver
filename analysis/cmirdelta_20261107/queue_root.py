"""根の比べの待ち行列 (続きから回せる)。58 問 (60 秒) で def・cd・cdg の根 (ノード上限 1) と HiGHS の根を測る。
使い方: python queue_root.py  (root58.jsonl に足す。HiGHS の根は modkvb_20261107/root58.jsonl を使う。済んだ (問題, 設定) は飛ばす)"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
MITT = sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
def done(out):
    s = set()
    if (D / out).exists():
        for l in open(D / out):
            try:
                r = json.loads(l); s.add((r["problem"], r["cfg"]))
            except Exception:
                pass
    return s
for probs, tl, out in [(PROBS, 60, "root58.jsonl")]:
    for p in probs:
        for cfg in ["def", "cd", "cdg"]:
            if (p, cfg) in done(out):
                continue
            if cfg == "highs":
                cmd = [sys.executable, str(D / "highs_root.py"), p, str(tl), out]
            else:
                cmd = [sys.executable, str(D / "run.py"), cfg, p, str(tl), "1", out, "root"]
            try:
                subprocess.run(cmd, timeout=tl + 600)
            except subprocess.TimeoutExpired:
                with open(D / out, "a") as f:
                    f.write(json.dumps({"problem": p, "cfg": cfg, "tl": tl, "root": True, "result": {"status": "killed"}}) + "\n")
