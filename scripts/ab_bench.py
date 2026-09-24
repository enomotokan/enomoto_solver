"""Interleaved A/B wall-clock benchmark of two builds of this crate over the
cached NETLIB problems (`.netlib_cache`, see docs/netlib-data.md).

Each arm is a Python interpreter (typically a venv) that has its own build of
`enomoto_solver._core` installed (`maturin develop --release`). For every
round and every problem, one subprocess per arm solves the problem `reps`
times in-process (the model is rebuilt before each solve, outside the timed
region) and reports the per-solve times; the arm order alternates between
rounds so slow drift of the machine does not favour either arm.

Usage:
    python scripts/ab_bench.py --base /path/venv_a/bin/python \
        --new /path/venv_b/bin/python --rounds 3 --out ab.json [--only NAME ...]

Summary (median over all samples per problem and arm) is printed and the raw
samples written to `--out`.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

WORKER = r"""
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
name, mps, reps = sys.argv[1], sys.argv[2], int(sys.argv[3])
h, lp = bh._load_lp(Path(mps))
out = {"name": name, "times": []}
for _ in range(reps):
    model, _, _ = bh._build_our_model(lp)
    t0 = time.perf_counter()
    r = model.solve(root_solver=None)
    out["times"].append(time.perf_counter() - t0)
    out["status"] = r["status"]
    out["obj"] = r["objective"]
print(json.dumps(out))
"""


def run_arm(python: str, name: str, mps: Path, reps: int, timeout: float,
            env: dict[str, str] | None = None) -> dict:
    try:
        cp = subprocess.run(
            [python, "-c", WORKER, name, str(mps), str(reps)],
            capture_output=True, text=True, timeout=timeout, cwd="/",
            env={**os.environ, **(env or {})},
        )
    except subprocess.TimeoutExpired:
        return {"name": name, "times": [], "status": "timeout"}
    for line in reversed(cp.stdout.splitlines()):
        if line.startswith("{"):
            return json.loads(line)
    return {"name": name, "times": [], "status": "error", "stderr": cp.stderr[-2000:]}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--new", required=True)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--min-sample-time", type=float, default=0.5,
                    help="small problems are solved repeatedly within one process until about this much time is spent")
    ap.add_argument("--timeout", type=float, default=300.0)
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".netlib_cache")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--base-env", nargs="*", default=[], metavar="KEY=VAL",
                    help="extra environment variables for the base arm")
    ap.add_argument("--new-env", nargs="*", default=[], metavar="KEY=VAL",
                    help="extra environment variables for the new arm")
    args = ap.parse_args()

    names = [l.strip() for l in (args.cache_dir / "problems.txt").read_text().splitlines() if l.strip()]
    if args.only:
        names = [n for n in names if n in set(args.only)]
    arms = {"base": args.base, "new": args.new}
    arm_env = {"base": dict(kv.split("=", 1) for kv in args.base_env),
               "new": dict(kv.split("=", 1) for kv in args.new_env)}
    samples: dict[str, dict[str, list[float]]] = {n: {"base": [], "new": []} for n in names}
    info: dict[str, dict[str, dict]] = {n: {} for n in names}
    reps: dict[str, int] = {}

    # Calibration pass (discarded): one cold solve per arm and problem sets
    # the per-process repetition count, so every measured sample is warm.
    for n in names:
        mps = args.cache_dir / "mps" / f"{n}.mps"
        if not mps.exists() or mps.stat().st_size == 0:
            continue
        ts = []
        for arm in ("base", "new"):
            r = run_arm(arms[arm], n, mps, 1, args.timeout, arm_env[arm])
            ts.extend(r["times"])
        t = min(ts, default=1.0)
        reps[n] = max(1, min(100, int(args.min_sample_time / max(t, 1e-4))))

    for rnd in range(args.rounds):
        order = ["base", "new"] if rnd % 2 == 0 else ["new", "base"]
        t_round = time.time()
        for n in names:
            mps = args.cache_dir / "mps" / f"{n}.mps"
            if not mps.exists() or mps.stat().st_size == 0:
                continue
            for arm in order:
                r = run_arm(arms[arm], n, mps, reps.get(n, 1), args.timeout, arm_env[arm])
                samples[n][arm].extend(r["times"])
                info[n][arm] = {"status": r.get("status"), "obj": r.get("obj")}
        print(f"round {rnd + 1}/{args.rounds} done in {time.time() - t_round:.0f}s", file=sys.stderr, flush=True)
        args.out.write_text(json.dumps({"samples": samples, "info": info}, indent=1))

    tot_b = tot_n = 0.0
    rows = []
    for n in names:
        b, a = samples[n]["base"], samples[n]["new"]
        if not b or not a:
            print(f"{n:<10} missing samples {info[n]}")
            continue
        mb, ma = statistics.median(b), statistics.median(a)
        tot_b += mb
        tot_n += ma
        ib, ia = info[n].get("base", {}), info[n].get("new", {})
        flag = ""
        if ib.get("status") != ia.get("status"):
            flag = " STATUS"
        elif ib.get("obj") is not None and ia.get("obj") is not None:
            if abs(ib["obj"] - ia["obj"]) > 1e-6 * max(1.0, abs(ib["obj"])):
                flag = " OBJ"
        rows.append((n, mb, ma, ma / mb - 1.0, flag))
    rows.sort(key=lambda r: -r[3])
    print(f"{'problem':<10} {'base':>9} {'new':>9} {'change':>8}")
    for n, mb, ma, ch, flag in rows:
        print(f"{n:<10} {mb:9.4f} {ma:9.4f} {ch*100:+7.1f}%{flag}")
    print(f"TOTAL base={tot_b:.3f}s new={tot_n:.3f}s change={(tot_n/tot_b-1)*100:+.2f}%")
    if rows:
        gm = math.exp(sum(math.log(r[2] / r[1]) for r in rows) / len(rows))
        print(f"GEOMEAN ratio new/base={gm:.4f} change={(gm-1)*100:+.2f}%")
    print(f"regressions >10%: {[r[0] for r in rows if r[3] > 0.10]}")


if __name__ == "__main__":
    main()
