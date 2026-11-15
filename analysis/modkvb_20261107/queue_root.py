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
def side_jobs():
    """side_jobs.txt の行 (シェルのコマンド) を 1 つずつ回す (計測の合間の割り込み。済んだ行は side_done.txt へ)"""
    f = D / "side_jobs.txt"
    while f.exists():
        lines = [l for l in f.read_text().splitlines() if l.strip()]
        if not lines:
            return
        cmd = lines[0]
        f.write_text("\n".join(lines[1:]) + ("\n" if lines[1:] else ""))
        subprocess.run(cmd, shell=True, cwd=D)
        with open(D / "side_done.txt", "a") as g:
            g.write(cmd + "\n")
def run(p, cfg, tl, out):
    side_jobs()
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
# Mittelmann: mk を先に回し、mk の分離が候補を 1 本も出さなかった問題 (分離の結果は既定と同じ) は def・HiGHS を省く (時間の節約)。
# 時間の上限は始めの 22 件は 120 秒、その後は 90 秒
for p in MITT:
    run(p, "mk", 90, "rootmitt.jsonl")
    if mk_cands("rootmitt.jsonl").get(p, 0) > 0:
        run(p, "def", 90, "rootmitt.jsonl")
# HiGHS は mk と def の根の下界が違った問題だけ (時間の節約)
B = {}
for l in open(D / "rootmitt.jsonl"):
    r = json.loads(l); B.setdefault(r["problem"], {})[r["cfg"]] = r["result"].get("bound")
for p in MITT:
    b = B.get(p, {})
    if isinstance(b.get("mk"), (int, float)) and isinstance(b.get("def"), (int, float)) and abs(b["mk"] - b["def"]) > 1e-6 * max(1, abs(b["def"])):
        run(p, "highs", 90, "rootmitt.jsonl")
side_jobs()
