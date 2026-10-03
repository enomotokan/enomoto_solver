"""Interleaved A/B wall-clock benchmark of two builds of this crate over the
Kennington and Mittelmann problems (the large-problem counterpart of
`ab_bench.py`, which covers Netlib).

The MPS files are expected uncompressed in `.kennington_cache/mps/` and
`.mittelmann_cache/mps/` (see scripts/paper_bench/prepare_data.py; the
Mittelmann problems are decompressed once into `mps/`). Each solve runs in a
fresh process of the arm's Python (`--base` / `--new`, each with its own build
of `enomoto_solver._core`), with the same setting as the paper benchmark
(`distinguish_infeasible_unbounded=True`). Only `model.solve()` is timed; a
solve still running `--timeout` seconds after model building finished is
killed and recorded as a timeout. The arm order alternates between rounds.

Results are appended to `--out` after every solve, and finished samples are
skipped when the same command is run again (resumable).

Usage:
    python scripts/ab_bench_large.py --base /venv_a/bin/python --new /venv_b/bin/python \
        --out ab_large.json [--rounds 1] [--only NAME ...] [--timeout 600]
"""
from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import threading
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

KENNINGTON = ["cre-a", "cre-b", "cre-c", "cre-d", "ken-07", "ken-11", "ken-13", "ken-18",
              "osa-07", "osa-14", "osa-30", "osa-60", "pds-02", "pds-06", "pds-10", "pds-20"]
# Mittelmann LPopt problems that at least one of ENOMOTO/HiGHS/CLP/SoPlex solved
# within 600 s in the paper benchmark (benchmarks/paper/summary.md).
MITTELMANN = ["cont1", "datt256", "ex10", "fome13", "irish-electricity", "Linf_520c", "neos",
              "neos-5052403-cygnet", "neos-5251015", "ns1688926", "nug08-3rd", "pds-100",
              "physiciansched3-3", "qap15", "rail4284", "rmine15", "s250r10", "scpm1", "square41",
              "stormG2_1000", "supportcase10", "woodlands09"]

WORKER = r"""
import json, math, sys, time
import highspy
from enomoto_solver import _core
mps = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
st = h.readModel(mps)
lp = h.getLp(); del h
m = _core.PyModel()
n = lp.num_col_
cl, cu = list(lp.col_lower_), list(lp.col_upper_)
for j in range(n):
    m.add_variable("continuous", float(cl[j]), float(cu[j]))
m.set_objective([(j, float(c)) for j, c in enumerate(list(lp.col_cost_)) if c != 0.0], float(lp.offset_),
                "maximize" if "kMaximize" in str(lp.sense_) else "minimize")
rows = [[] for _ in range(lp.num_row_)]
am = lp.a_matrix_
s, ix, v = list(am.start_), list(am.index_), list(am.value_)
if "kColwise" in str(am.format_):
    for j in range(n):
        for k in range(s[j], s[j + 1]):
            if v[k] != 0.0: rows[ix[k]].append((j, v[k]))
else:
    for i in range(lp.num_row_):
        for k in range(s[i], s[i + 1]):
            if v[k] != 0.0: rows[i].append((ix[k], v[k]))
del s, ix, v
rl, ru = list(lp.row_lower_), list(lp.row_upper_)
for i in range(lp.num_row_):
    lo, hi, t = rl[i], ru[i], rows[i]; rows[i] = None
    if math.isinf(lo) and math.isinf(hi): continue
    if not math.isinf(lo) and abs(hi - lo) < 1e-12:
        m.add_constraint(t, "==", float(lo)); continue
    if not math.isinf(hi): m.add_constraint(t, "<=", float(hi))
    if not math.isinf(lo): m.add_constraint(t, ">=", float(lo))
del lp, rows
print(json.dumps({"built": True}), flush=True)
t0 = time.perf_counter()
out = m.solve(root_solver=None, distinguish_infeasible_unbounded=True)
print(json.dumps({"time": time.perf_counter() - t0, "status": out["status"], "obj": out.get("objective")}), flush=True)
"""


def problem_path(name: str) -> Path:
    if name in KENNINGTON:
        return REPO_ROOT / ".kennington_cache" / "mps" / f"{name}.mps"
    return REPO_ROOT / ".mittelmann_cache" / "mps" / f"{name}.mps"


def run_one(python: str, mps: Path, timeout: float, env: dict[str, str]) -> dict:
    p = subprocess.Popen([python, "-c", WORKER, str(mps)], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         text=True, cwd="/", env={**os.environ, **env})
    lines: list[str] = []
    err: list[str] = []
    built = threading.Event()

    def _out() -> None:
        for line in p.stdout:  # type: ignore[union-attr]
            lines.append(line.strip())
            if '"built"' in line:
                built.set()

    th = threading.Thread(target=_out, daemon=True)
    th_e = threading.Thread(target=lambda: err.append(p.stderr.read()), daemon=True)  # type: ignore[union-attr]
    th.start()
    th_e.start()
    while not built.is_set() and p.poll() is None:
        time.sleep(0.05)
    try:
        p.wait(timeout=timeout + 5.0)
    except subprocess.TimeoutExpired:
        p.kill()
        p.wait()
        return {"status": "timeout", "time": None}
    th.join(5)
    th_e.join(5)
    res = [json.loads(l) for l in lines if l.startswith("{") and '"built"' not in l]
    if p.returncode != 0 or not res:
        return {"status": "crash", "time": None, "stderr": "".join(err)[-1500:]}
    r = res[-1]
    if r["time"] > timeout:
        r["status"] = "timeout"
    tail = "".join(err)[-3000:]
    if tail:
        r["stderr"] = tail
    return r


def sgm(ts: list[float], shift: float) -> float:
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--new", required=True)
    ap.add_argument("--rounds", type=int, default=1)
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--skip", nargs="*", default=[])
    ap.add_argument("--arms", nargs="*", default=["base", "new"])
    ap.add_argument("--base-env", nargs="*", default=[], metavar="KEY=VAL")
    ap.add_argument("--new-env", nargs="*", default=[], metavar="KEY=VAL")
    ap.add_argument("--report", action="store_true")
    args = ap.parse_args()

    names = KENNINGTON + MITTELMANN
    if args.only:
        names = [n for n in names if n in set(args.only)]
    names = [n for n in names if n not in set(args.skip)]
    data = json.loads(args.out.read_text()) if args.out.exists() else {}
    pythons = {"base": args.base, "new": args.new}
    envs = {"base": dict(kv.split("=", 1) for kv in args.base_env),
            "new": dict(kv.split("=", 1) for kv in args.new_env)}
    if not args.report:
        for rnd in range(args.rounds):
            order = args.arms if rnd % 2 == 0 else list(reversed(args.arms))
            for n in names:
                for arm in order:
                    runs = data.setdefault(n, {}).setdefault(arm, [])
                    if len(runs) > rnd:
                        continue
                    r = run_one(pythons[arm], problem_path(n), args.timeout, envs[arm])
                    runs.append(r)
                    args.out.write_text(json.dumps(data, indent=1))
                    print(f"{n:22s} {arm:5s} #{rnd + 1} {r['status']:10s} {r.get('time') or float('nan'):9.3f} "
                          f"obj={r.get('obj')}", flush=True)

    def med(runs: list[dict]) -> float | None:
        ts = sorted(r["time"] if r.get("status") == "optimal" and r.get("time") is not None else args.timeout
                    for r in runs)
        return ts[len(ts) // 2] if ts else None

    tb, tn = [], []
    print(f"\n{'problem':22s} {'base':>9s} {'new':>9s} {'change':>8s}")
    for n in names:
        b, nn = med(data.get(n, {}).get("base", [])), med(data.get(n, {}).get("new", []))
        if b is None or nn is None:
            continue
        tb.append(b)
        tn.append(nn)
        print(f"{n:22s} {b:9.3f} {nn:9.3f} {100 * (nn / b - 1):+7.1f}%")
    if tb:
        for shift in (10.0, 1.0):
            sb, sn = sgm(tb, shift), sgm(tn, shift)
            print(f"shifted geomean ({shift:g} s): base {sb:.4f} new {sn:.4f} ({100 * (sn / sb - 1):+.2f}%)")


if __name__ == "__main__":
    main()
