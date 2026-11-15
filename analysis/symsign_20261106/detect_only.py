"""対称性の検出だけを数える (2026-11-06)。1 問を 5 つの設定 (def・zero・sign・scale・both) で、最初の検出の直後に止めて
(`ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM`) ログから生成元の数・軌道の数・検出の時間などを拾い、detect_<集合>.jsonl に書く。
使い方: python detect_only.py 集合(p58|mitt) [並列の数 (既定 4)]"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "zero": {"ENOMOTO_MIP_SYMZERO": "1"}, "sign": {"ENOMOTO_MIP_SYMSIGN": "1"}, "scale": {"ENOMOTO_MIP_SYMSCALE": "1"},
        "both": {"ENOMOTO_MIP_SYMSIGN": "1", "ENOMOTO_MIP_SYMSCALE": "1"}}
which = sys.argv[1]; par = int(sys.argv[2]) if len(sys.argv) > 2 else 4
probs = PROBS if which == "p58" else sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
OUT = D / f"detect_{which}.jsonl"
done = set()
if OUT.exists():
    for l in open(OUT):
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
TO = int(os.environ.get("DETECT_TIMEOUT", "400"))

def parse(err):
    r = {}
    m = re.search(r"MIP: symmetry: (\d+) generators, (\d+) orbits on (\d+) columns, (\d+) lex-leader rows \(([\d.]+)s.*?detect ([\d.]+)s", err)
    if m:
        r.update(gens=int(m[1]), orbits=int(m[2]), moved=int(m[3]), lex=int(m[4]), t_sym=float(m[5]), t_detect=float(m[6]))
    m = re.search(r"MIP: symmetry: none \(([\d.]+)s.*?detect ([\d.]+)s", err)
    if m:
        r.update(gens=0, orbits=0, moved=0, lex=0, t_sym=float(m[1]), t_detect=float(m[2]))
    if "symmetry: skipped" in err: r["skipped"] = True
    m = re.search(r"flipped (\d+), scaled (\d+), tie rows doubled (\d+) of (\d+) rows", err)
    if m: r.update(flipped=int(m[1]), scaled=int(m[2]), ties=int(m[3]), rows=int(m[4]))
    m = re.search(r"HO3_SYMNORM flipped (\d+) of (\d+) rows", err)
    if m: r.update(flipped=int(m[1]), rows=int(m[2]))
    m = re.search(r"refinement ([\d.]+)s, work", err)
    if m: r["t_refine"] = float(m[1])
    m = re.search(r"(\d+) full orbitopes", err)
    if m: r["orbitopes"] = int(m[1])
    m = re.search(r"twins as orbitopes: (\d+) classes", err)
    if m: r["twin_orbitopes"] = int(m[1])
    m = re.search(r"stopped after the symmetry detection \(ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM, ([\d.]+)s\)", err)
    if m: r["t_presolve_sym"] = float(m[1]); r["stopped"] = True
    return r

def one(job):
    p, cfg = job
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1", **CFGS[cfg])
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=TO)
        err, rc = pr.stderr, pr.returncode
    except subprocess.TimeoutExpired as e:
        err, rc = (e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr or "")), "timeout"
    r = {"problem": p, "cfg": cfg, "rc": rc, "wall": round(time.time() - t0, 2), **parse(err)}
    if not r.get("stopped") and rc != "timeout":
        r["tail"] = err[-300:]
    return r

jobs = [(p, c) for p in probs for c in CFGS if (p, c) not in done]
with ThreadPoolExecutor(par) as ex, open(OUT, "a") as f:
    for r in ex.map(one, jobs):
        f.write(json.dumps(r) + "\n"); f.flush()
        print(r["problem"], r["cfg"], r.get("gens"), r.get("orbits"), r.get("t_detect"), r.get("ties"), r["wall"], flush=True)
