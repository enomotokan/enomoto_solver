"""6 章の待ち行列 (queue4.py の続きを含む、続きから回せる)。
1. queue4 の残り: Mittelmann の効く 8 問の部分を外した根 (rootmitt.jsonl)
2. zerohalf との重なり (根、種 1・2): 58 問で mk50z (数え上げ)・mk50cg (CG の式)・mk50cgb (CG の式 + 従来)、comp21-2idx・mushroom-best でも
3. 58 問の 60 秒 (種 1・2、b58.jsonl): nozh6・mk50nozh6 (既定・mk50 は済み)
4. Mittelmann 186 問の 60 秒 (種 1、bmitt.jsonl): def6・mk50z
5. mk50 の長い計測 (queue4 の 2)
6. Mittelmann 186 問の 60 秒 (種 2)
使い方: python queue5.py"""
import json, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
MITT = sorted({p for s in json.loads((WT / "analysis/mitt600_20261103/shards2.json").read_text())["shards"] for p in s})
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
PM = ["comp21-2idx", "comp07-2idx", "enlight_hard", "mushroom-best", "mzzv11", "neos-3024952-loue", "neos-3046615-murg", "milo-v12-6-r2-40-1"]
for rep in [1, 2]:
    for p in PM:
        for cfg in ["def5", "mk50b", "nozh", "mk50nozh", "mk50novb", "mk50nocut", "mk50k2"]:
            run(p, cfg, 90, rep, "rootmitt.jsonl", True)
# queue4 の 1 の残り (58 問の根の 2 回目)
P58 = ["misc07", "fiber", "mzzv42z", "p0201", "nu25-pr12", "swath1", "rout", "blend2", "mc11", "h80x6320d", "qnet1", "neos5"]
for p in PROBS:
    for cfg in ["nozh", "mk50nozh"]:
        run(p, cfg, 60, 2, "root58.jsonl", True)
for p in P58:
    for cfg in ["mk50novb", "mk50nocut", "mk50k2"]:
        run(p, cfg, 60, 2, "root58.jsonl", True)
for rep in [1, 2]:
    for p in PROBS:
        for cfg in ["mk50z", "mk50cg", "mk50cgb"]:
            run(p, cfg, 60, rep, "root58.jsonl", True)
    for p in ["comp21-2idx", "mushroom-best"]:
        for cfg in ["mk50z", "mk50cg", "mk50cgb"]:
            run(p, cfg, 90, rep, "rootmitt.jsonl", True)
for rep in [1, 2]:
    for p in PROBS:
        for cfg in ["nozh6", "mk50nozh6"]:
            run(p, cfg, 60, rep, "b58.jsonl")
for p in MITT:
    for cfg in ["def6", "mk50z"]:
        run(p, cfg, 60, 1, "bmitt.jsonl")
for rep in [1, 2]:
    for p in ["comp21-2idx", "mzzv42z", "mushroom-best"]:
        run(p, "mk50b", 600, rep, "long.jsonl")
    for p in ["neos-3024952-loue", "milo-v12-6-r2-40-1"]:
        for cfg in ["def5", "mk50b"]:
            run(p, cfg, 600, rep, "long.jsonl")
for p in MITT:
    for cfg in ["def6", "mk50z"]:
        run(p, cfg, 60, 2, "bmitt.jsonl")
