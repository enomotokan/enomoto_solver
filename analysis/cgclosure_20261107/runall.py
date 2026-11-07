"""閉包の計測を並べて回す (2026-11-07)。1 本あたり HiGHS 1 スレッド、同時に --par 本。済んだ (問題, モード) は closures.jsonl を見て飛ばす (止まっても続きから)。
使い方: python runall.py --modes cg,mod2,modk --tsep 20 --tmax 300 --par 4 問題..."""
import argparse, json, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
ap = argparse.ArgumentParser()
ap.add_argument("--modes", default="cg,mod2,modk")
ap.add_argument("--tsep", type=float, default=20)
ap.add_argument("--tmax", type=float, default=300)
ap.add_argument("--par", type=int, default=4)
ap.add_argument("--out", default="closures.jsonl")
ap.add_argument("--extra", default="", help="cgclosure.py に足す引数 (空白区切り)")
ap.add_argument("probs", nargs="+")
a = ap.parse_args()
OUT = D / a.out
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["name"], r["mode"]))
jobs = [(p, m) for p in a.probs for m in a.modes.split(",") if (p, m) not in done]
(D / "logs").mkdir(exist_ok=True)


def run(job):
    p, m = job
    sol = Path("/tmp/sols") / f"{p}.sol.gz"
    cmd = [sys.executable, "-I", str(D / "cgclosure.py"), str(WT / ".miplib_cache" / f"{p}.mps"), m, "--tsep", str(a.tsep), "--tmax", str(a.tmax),
           "--name", p, "--out", str(OUT)]
    if sol.exists():
        cmd += ["--sol", str(sol)]
    cmd += a.extra.split()
    t0 = time.time()
    tag = "" if a.out == "closures.jsonl" else Path(a.out).stem + "_"
    with open(D / "logs" / f"{tag}{p}_{m}.log", "w") as f:
        try:
            r = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=f, timeout=a.tmax + 900)
            rc = r.returncode
        except subprocess.TimeoutExpired:
            rc = "timeout"
    if rc != 0:
        with open(OUT, "a") as f:
            f.write(json.dumps({"name": p, "mode": m, "fail": str(rc), "time": round(time.time() - t0, 1)}) + "\n")
    print(p, m, rc, round(time.time() - t0, 1), flush=True)


with ThreadPoolExecutor(a.par) as ex:
    list(ex.map(run, jobs))
