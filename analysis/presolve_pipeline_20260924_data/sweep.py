"""Per-problem breakdown sweep.

For every NETLIB problem two fresh processes of the given python:
  1. clean: build model once, solve repeatedly (until --budget s, max 30) -> times
  2. prof : same, but with profiling env vars set from process start, stderr parsed
     (PROF_PIPE / PROF_PRESOLVE / PROF_PHASES_EXT / PRESOLVE_SIZE / PRESOLVE_HASH)

Usage: python sweep.py --python PY --out out.json [--only a b] [--env K=V ...]
       [--prof-env K=V ...] [--no-clean]
"""
import argparse, json, os, statistics, subprocess, sys, time
from pathlib import Path

REPO = Path("/home/user/enomoto_solver")
WORKER = r"""
import json, sys, time, os, tempfile
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
name, mps, mode, reps, budget = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), float(sys.argv[5])
h, lp = bh._load_lp(Path(mps))
model, ncons, nnz = bh._build_our_model(lp)
out = {"name": name, "n": lp.num_col_, "m": ncons, "nnz": nnz, "times": []}
if mode == "clean":
    t_start = time.perf_counter()
    for k in range(reps):
        t0 = time.perf_counter()
        r = model.solve(root_solver=None)
        dt = time.perf_counter() - t0
        out["times"].append(dt)
        out["status"] = r["status"]; out["obj"] = r["objective"]
        if time.perf_counter() - t_start > budget and k >= 2:
            break
    out["first"] = out["times"][0]
else:
    runs = []
    t_start = time.perf_counter()
    for k in range(reps):
        if time.perf_counter() - t_start > budget and k >= 3:
            break
        tf = tempfile.TemporaryFile(mode="w+")
        saved = os.dup(2); os.dup2(tf.fileno(), 2)
        t0 = time.perf_counter()
        r = model.solve(root_solver=None)
        dt = time.perf_counter() - t0
        out["status"] = r["status"]; out["obj"] = r["objective"]
        os.dup2(saved, 2); os.close(saved)
        tf.seek(0); txt = tf.read(); tf.close()
        steps = {}; extra = {}; pipe = {}; pipe_cnt = {}; ph = {}
        for line in txt.splitlines():
            s = line.strip()
            if s.startswith("PROF_PRESOLVE total"):
                extra["presolve_total_eprint"] = float(s.split()[-1][:-2])
            elif s.startswith("PROF_PRESOLVE ") and s.endswith("ms"):
                parts = s.split(); label = " ".join(parts[1:-1]); val = float(parts[-1][:-2])
                steps[label] = steps.get(label, 0.0) + val
            elif s.startswith("PROF_PHASES_EXT wall="):
                toks = s.split()
                extra["ext_wall"] = float(toks[1].split("=")[1][:-2])
                extra["iters"] = int(toks[2].split("=")[1])
                for t in toks:
                    if t.startswith("refactor_count="):
                        extra["refc"] = int(t.split("=")[1])
            elif s.startswith("PRESOLVE_SIZE"):
                for t in s.split()[1:]:
                    k, v = t.split("="); extra[k] = int(v)
            elif s.startswith("PRESOLVE_HASH"):
                toks = s.split(); extra["hash"] = toks[1]
                for t in toks[2:]:
                    k, v = t.split("="); extra["ps_" + k] = int(v)
            elif s.startswith("PROF_PIPE"):
                for t in s.split()[1:]:
                    k, v = t.split("=", 1); us, cnt = v.split("/")
                    pipe[k] = float(us); pipe_cnt[k] = int(cnt)
            elif s.endswith("%)") and "ms" in s and not s.startswith("PROF"):
                toks = s.split()
                try:
                    ph[toks[0]] = float(toks[1][:-2])
                except Exception:
                    pass
        runs.append({"dt": dt, "steps": steps, "ph": ph, "pipe": pipe, "pipe_cnt": pipe_cnt, **extra})
    out["prof"] = runs
print(json.dumps(out))
"""


def run(python, n, mps, mode, reps, budget, env):
    cp = subprocess.run([python, "-c", WORKER, n, str(mps), mode, str(reps), str(budget)],
                        capture_output=True, text=True, env=env, cwd="/", timeout=1200)
    line = [l for l in cp.stdout.splitlines() if l.startswith("{")]
    if not line:
        print(n, mode, "ERROR", cp.stderr[-800:], file=sys.stderr)
        return None
    return json.loads(line[-1])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--python", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--env", nargs="*", default=[])
    ap.add_argument("--prof-env", nargs="*", default=["ENOMOTO_PROF_PIPE=1", "ENOMOTO_DEBUG_PRESOLVE_SIZE=1"])
    ap.add_argument("--budget", type=float, default=0.6)
    ap.add_argument("--max-reps", type=int, default=30)
    ap.add_argument("--prof-reps", type=int, default=3)
    ap.add_argument("--no-clean", action="store_true")
    ap.add_argument("--no-prof", action="store_true")
    a = ap.parse_args()
    names = [l.strip() for l in (REPO / ".netlib_cache/problems.txt").read_text().splitlines() if l.strip()]
    if a.only:
        names = [n for n in names if n in set(a.only)]
    env = {**os.environ, **dict(kv.split("=", 1) for kv in a.env)}
    penv = {**env, **dict(kv.split("=", 1) for kv in a.prof_env)}
    res = []
    for n in names:
        mps = REPO / ".netlib_cache/mps" / f"{n}.mps"
        t0 = time.time()
        d = {"name": n}
        if not a.no_clean:
            c = run(a.python, n, mps, "clean", a.max_reps, a.budget, env)
            if c is None:
                continue
            d.update(c)
            d["med"] = statistics.median(c["times"]); d["min"] = min(c["times"])
        if not a.no_prof:
            p = run(a.python, n, mps, "prof", a.prof_reps, a.budget, penv)
            if p is None:
                continue
            d["prof"] = p["prof"]
            for k in ("n", "m", "nnz", "status", "obj"):
                d.setdefault(k, p.get(k))
        res.append(d)
        msg = f"{n:<10}"
        if "med" in d:
            msg += f" med={d['med']*1e3:9.3f}ms min={d['min']*1e3:9.3f} first={d['first']*1e3:9.3f} reps={len(d['times'])}"
        if "prof" in d:
            pp = d["prof"][-1]["pipe"]
            msg += f" | pipe: solve_mip={pp.get('solve_mip',0)/1e3:9.3f}ms presolve={pp.get('run_extended',0)/1e3:8.3f} ext_main={pp.get('ext_main',0)/1e3:8.3f}"
        print(msg + f" ({time.time()-t0:.0f}s)", file=sys.stderr, flush=True)
        Path(a.out).write_text(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
