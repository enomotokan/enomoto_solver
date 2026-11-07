"""5 章の待ち行列 (続きから回せる、wheel は /tmp/pyw5)。
1. どの部分が効くか (根、種 1・2): 58 問すべてで nozh (zerohalf を止めた既定)・mk50nozh (zerohalf を止めた mk50)、効いた/下がった 20 問で
   mk50novb・mk50nocut・mk50k2 (+ Mittelmann の 8 問は def5・mk50b も)。58 問は root58.jsonl (60 秒)、Mittelmann は rootmitt.jsonl (90 秒)。
2. mk50 の長い計測 (600 秒、種 1・2、long.jsonl): comp21-2idx・mzzv42z・mushroom-best は mk50b、neos-3024952-loue・milo-v12-6-r2-40-1 は def5・mk50b。
3. Mittelmann の 60 秒 (種 1、bmitt.jsonl): 根の比べで mk が候補を出した問題で def5・mk50b。
使い方: python queue4.py"""
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
P58 = ["misc07", "fiber", "mzzv42z", "p0201", "nu25-pr12", "swath1", "rout", "blend2", "mc11", "h80x6320d", "qnet1", "neos5"]
PM = ["comp21-2idx", "comp07-2idx", "enlight_hard", "mushroom-best", "mzzv11", "neos-3024952-loue", "neos-3046615-murg", "milo-v12-6-r2-40-1"]
for rep in [1, 2]:
    for p in PROBS:
        for cfg in ["nozh", "mk50nozh"]:
            run(p, cfg, 60, rep, "root58.jsonl", True)
    for p in P58:
        for cfg in ["mk50novb", "mk50nocut", "mk50k2"]:
            run(p, cfg, 60, rep, "root58.jsonl", True)
    for p in PM:
        for cfg in ["def5", "mk50b", "nozh", "mk50nozh", "mk50novb", "mk50nocut", "mk50k2"]:
            run(p, cfg, 90, rep, "rootmitt.jsonl", True)
for rep in [1, 2]:
    for p in ["comp21-2idx", "mzzv42z", "mushroom-best"]:
        run(p, "mk50b", 600, rep, "long.jsonl")
    for p in ["neos-3024952-loue", "milo-v12-6-r2-40-1"]:
        for cfg in ["def5", "mk50b"]:
            run(p, cfg, 600, rep, "long.jsonl")
cands = {}
for l in open(D / "rootmitt.jsonl"):
    r = json.loads(l)
    if r["cfg"] == "mk" and r.get("sep", {}).get("cands", 0) > 0:
        cands[r["problem"]] = 1
for p in sorted(cands):
    for cfg in ["def5", "mk50b"]:
        run(p, cfg, 60, 1, "bmitt.jsonl")
