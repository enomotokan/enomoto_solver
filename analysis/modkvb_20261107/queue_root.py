"""根の比べの待ち行列 (続きから回せる)。58 問 (60 秒) と Mittelmann 186 問 (120 秒) で def・mk の根 (ノード上限 1) と HiGHS の根を測る。
HiGHS の根は Mittelmann では mk が候補を出した問題だけ。
使い方: python queue_root.py  (root58.jsonl / rootmitt.jsonl に足す。済んだ (問題, 設定) は飛ばす)"""
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
def mk_cands(out):
    c = {}
    if (D / out).exists():
        for l in open(D / out):
            r = json.loads(l)
            if r["cfg"] == "mk": c[r["problem"]] = r.get("sep", {}).get("cands", 0)
    return c
def run(p, cfg, tl, out):
    if (p, cfg) in done(out):
        return
    if cfg == "highs":
        cmd = [sys.executable, str(D / "highs_root.py"), p, str(tl), out]
    else:
        cmd = [sys.executable, str(D / "run.py"), cfg, p, str(tl), "1", out, "root"]
    try:
        subprocess.run(cmd, timeout=tl + 600)
    except subprocess.TimeoutExpired:
        with open(D / out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "tl": tl, "root": True, "result": {"status": "killed"}}) + "\n")
# 58 問は 3 つとも。Mittelmann は def・mk を先に全部、HiGHS は mk が候補を出した問題だけ (時間の節約)
for p in PROBS:
    for cfg in ["def", "mk", "highs"]:
        run(p, cfg, 60, "root58.jsonl")
for p in MITT:
    for cfg in ["def", "mk"]:
        run(p, cfg, 120, "rootmitt.jsonl")
mc = mk_cands("rootmitt.jsonl")
for p in MITT:
    if mc.get(p, 0) > 0:
        run(p, "highs", 120, "rootmitt.jsonl")
