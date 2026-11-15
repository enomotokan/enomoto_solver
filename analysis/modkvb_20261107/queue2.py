"""queue_root.py の後の待ち行列 (続きから回せる)。
1. 58 問の根の 2 回目 (def・mk、種 2、root58.jsonl) — 揺れを見る
2. 58 問の 60 秒 (def・mk、種 1・2、b58.jsonl) — 害を見る
3. long.txt の行 (問題 時間) の def・mc・mk を種 1・2 (long.jsonl) — 効く問題の確かめ
使い方: python queue2.py"""
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
for p in PROBS:
    for cfg in ["def", "mk"]:
        run(p, cfg, 60, 2, "root58.jsonl", True)
for rep in [1, 2]:
    for p in PROBS:
        for cfg in ["def", "mk"]:
            run(p, cfg, 60, rep, "b58.jsonl")
if (D / "long.txt").exists():
    for rep in [1, 2]:
        for l in open(D / "long.txt"):
            if l.strip() and not l.startswith("#"):
                p, tl = l.split()[:2]
                for cfg in ["def", "mc", "mk"]:
                    run(p, cfg, int(tl), rep, "long.jsonl")
