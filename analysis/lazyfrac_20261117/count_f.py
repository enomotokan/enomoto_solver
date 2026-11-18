"""外せる行の非零の割合 f を数える (木に入るときに診断を出して終える `ENOMOTO_MIP_DIAG_GENLAZY=exit`)。
GENLAZY の点数 τ (`F_TAU`、既定 0) と CLQ_LAZY の文字の和 < θ (`F_CLQ_THETA`、既定 0.1) の候補を同じ回で数える (CLQ は外さず数えるだけ)。
使い方: python count_f.py [58|mitt|問題名,...]。1 問 1 行を count_f.jsonl に足す (途中から再開できる)。`CF_THREADS` スレッド (既定 4)、`CF_PART=i/n` で分けて並べて回せる、壁時計 `CF_TL` 秒 (既定 300) で打ち切る。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
which = sys.argv[1] if len(sys.argv) > 1 else "58"
if which == "58":
    probs = PROBS
elif which == "mitt":
    probs = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
else:
    probs = which.split(",")
PART = os.environ.get("CF_PART")  # "i/n" なら i 番目の分だけ (並べて回す)
if PART:
    i, n = map(int, PART.split("/")); probs = probs[i::n]
THR = os.environ.get("CF_THREADS", "4")
OUT = D / (f"count_f_{PART.replace('/', 'of')}.jsonl" if PART else "count_f.jsonl")
TAU = os.environ.get("F_TAU", "0"); TH = float(os.environ.get("F_CLQ_THETA", "0.1"))
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = float(os.environ.get("CF_TL", "300"))
for p in probs:
    if p in done:
        continue
    rec = {"set": which if which in ("58", "mitt") else "x", "problem": p, "tau": float(TAU), "clq_theta": TH}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        rec["error"] = f"fetch: {e}"[:200]; mps = None
    if mps is not None:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_GEN_LAZY="1", ENOMOTO_MIP_GEN_LAZY_MODE="score", ENOMOTO_T_MIP_GEN_LAZY_TAU=TAU, ENOMOTO_MIP_CLQ_LAZY="1",
                   ENOMOTO_T_MIP_CLQ_LAZY_TOL=str(1 - TH), ENOMOTO_MIP_DIAG_GENLAZY="exit", ENOMOTO_MIP_THREADS=THR)
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(int(TL)), "--threads", THR],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 60)
            for l in pr.stderr.splitlines():
                if l.startswith("GENLAZY {"):
                    d = json.loads(l[8:])
                    dn = {k: v["dropnnz"] for k, v in d.items() if isinstance(v, dict)}
                    rec.update(m=d["m"], nnz=d["nnz"], gen_dropnnz=sum(dn.values()), gen_f=sum(dn.values()) / max(1, d["nnz"]), gen_by_kind=dn,
                               gen_rows=sum(v["drop"] for v in d.values() if isinstance(v, dict)))
                elif l.startswith("CLQLAZY_F {"):
                    c = json.loads(l[10:]); rec.update(clq_f=c["f"], clq_f_all=c.get("f_all"), clq_rows=c["rows"])
            if "gen_f" not in rec:
                outs = [l for l in pr.stdout.splitlines() if l.startswith("{")]
                rec["status"] = json.loads(outs[-1]).get("status") if outs else f"rc {pr.returncode}"
                rec["note"] = "no tree (solved or stopped before the tree)"
        except subprocess.TimeoutExpired:
            rec["error"] = "timeout"
        rec["wall"] = round(time.time() - t0, 1)
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec.get("gen_f"), rec.get("clq_f"), rec.get("status") or rec.get("error"), rec.get("wall"), flush=True)
