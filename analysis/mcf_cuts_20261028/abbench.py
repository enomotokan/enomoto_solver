"""既定と MCF の試作 (ENOMOTO_MIP_SEPA_MCF=1) を、1 問 1 サブプロセス (scripts/mip_bench/bench.py --worker、1 スレッド) で
問題ごとに 既定 1 -> 試作 1 -> 既定 2 -> 試作 2 の順に並べて -j 並列で解く。結果は --out (JSON) に書き足す (中断しても再開できる)。

使い方: python analysis/mcf_cuts_20261028/abbench.py --out 結果.json [--tl 120] [-j 4] [--only 問題 ...] [--nodes N]
"""
import argparse, json, os, subprocess, sys, threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PROBS = ["beasleyC3", "beavma", "fiber", "g200x740", "germany50-UUM", "h50x2450", "h80x6320d", "k16x240b", "khb05250",
         "mc11", "mc7", "mc8", "n3700", "n3705", "n3707", "n3709", "n370b", "neos-1171737", "neos-4387871-tavua", "r50x360",
         "ta1-UUM", "ta2-UUE", "transportmoment", "cvs16r128-89"]
CFGS = {"def": {}, "mcf": {"ENOMOTO_MIP_SEPA_MCF": "1"}}

ap = argparse.ArgumentParser()
ap.add_argument("--out", required=True); ap.add_argument("--tl", type=float, default=120); ap.add_argument("-j", type=int, default=4)
ap.add_argument("--only", nargs="*"); ap.add_argument("--nodes", type=int); ap.add_argument("--reps", type=int, default=2)
ap.add_argument("--cfg", nargs="*", help="NAME=VAR=VAL[,VAR=VAL] で設定を足す")
a = ap.parse_args()
for c in a.cfg or []:
    name, rest = c.split("=", 1)
    CFGS[name] = dict(kv.split("=", 1) for kv in rest.split(",")) if rest else {}
out = Path(a.out)
res = json.loads(out.read_text()) if out.exists() else {}
lock = threading.Lock()
jobs = [(p, c, r) for p in (a.only or PROBS) for r in range(1, a.reps + 1) for c in CFGS if f"{p}|{c}|{r}" not in res]


def run(job):
    p, c, r = job
    cmd = [sys.executable, str(REPO / "scripts/mip_bench/bench.py"), "--worker", str(REPO / ".miplib_cache" / f"{p}.mps"), "--tl", str(a.tl), "--threads", "1"]
    if a.nodes:
        cmd += ["--nodes", str(a.nodes)]
    env = dict(os.environ, **CFGS[c])
    try:
        pr = subprocess.run(cmd, capture_output=True, text=True, timeout=a.tl * 1.5 + 120, env=env)
        o = next((json.loads(l) for l in reversed(pr.stdout.splitlines()) if l.startswith("{")), {"status": f"crash rc={pr.returncode}", "stderr": pr.stderr[-800:]})
    except subprocess.TimeoutExpired:
        o = {"status": "killed"}
    with lock:
        res[f"{p}|{c}|{r}"] = o
        out.write_text(json.dumps(res, indent=1))
    print(f"{p:20s} {c:4s} {r} {o.get('status'):>10s} t={o.get('time')} obj={o.get('objective')} bb={o.get('best_bound')} root={o.get('root_bound')} nodes={o.get('nodes')}", flush=True)


with ThreadPoolExecutor(a.j) as ex:
    list(ex.map(run, jobs))
