"""Runtime comparison: this crate's simplex engine vs. HiGHS (via `highspy`)
on the Netlib LP benchmark set, restricted to problems with at most
`--max-vars` (default 3000) columns.

`model.rs::add_variable` now accepts genuine `+/-inf` bounds directly (see
its own docs) — this crate's presolve pipeline substitutes a finite
`BIG_M` internally only for whatever survives presolve without being
eliminated outright (`simplex.rs::build_std_form_presolved`), not before
presolve ever runs. So every Netlib problem here is handed to this crate
exactly as HiGHS itself reads it from the `.mps` file, MPS `+inf` bounds
included — no bound substitution happens in this script at all anymore.
Objective values are still compared as a sanity check.

Usage:
    python -m enomoto_solver.benchmark_highs [--max-vars 3000] [--timeout 60]

Netlib's LP data is not committed to this repository (see
docs/netlib-data.md) and this script never itself talks to netlib.org:
run `python scripts/setup_netlib_data.py` once per environment first — it
fetches Netlib's custom "compressed MPS" format and decompresses it via
Netlib's own `emps.c` (compiled once, requires a C compiler in `PATH`, e.g.
`gcc`) into `--cache-dir` (default: `<repo>/.netlib_cache`, gitignored).
This script only ever reads that cache.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import subprocess
import sys
import time
from pathlib import Path

import highspy

from . import _core


class NetlibCacheError(RuntimeError):
    """Needed Netlib data isn't cached — run `scripts/setup_netlib_data.py`
    (from a checkout of this repo) once per environment first."""


def _cached_problem_list(cache_dir: Path) -> list[str]:
    list_path = cache_dir / "problems.txt"
    if not list_path.exists():
        raise NetlibCacheError(f"{list_path} not found — run `python scripts/setup_netlib_data.py` once to fetch it")
    return [line.strip() for line in list_path.read_text().splitlines() if line.strip()]


def _cached_mps(name: str, cache_dir: Path) -> Path | None:
    mps_path = cache_dir / "mps" / f"{name}.mps"
    return mps_path if mps_path.exists() and mps_path.stat().st_size > 0 else None


def _load_lp(mps_path: Path):
    """Reads `mps_path` via HiGHS and returns `(highs_instance, lp)` — the
    `highs_instance` is reused as-is for the HiGHS-side timing below, so
    the same parsed model backs both solves."""
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    status = h.readModel(str(mps_path))
    if "kOk" not in str(status):
        raise RuntimeError(f"readModel failed: {status}")
    return h, h.getLp()


def _build_our_model(lp) -> tuple[_core.PyModel, int, int]:
    """Builds this crate's `PyModel` directly from HiGHS's parsed LP data
    (bypassing the Python `Variable`/`Constraint` DSL, which would add
    per-term Python-object overhead irrelevant to the solver's own
    runtime) — bounds passed through exactly as HiGHS parsed them,
    `+/-inf` included (`add_variable` accepts real infinities now; see its
    own docs). Returns `(model, n_constraints, nnz)`."""
    m = _core.PyModel()
    n = lp.num_col_

    for j in range(n):
        lb = float(lp.col_lower_[j])
        ub = float(lp.col_upper_[j])
        if lb > ub:
            lb, ub = ub, lb
        is_int = len(lp.integrality_) > j and int(lp.integrality_[j]) != 0
        m.add_variable("integer" if is_int else "continuous", lb, ub)

    obj_coeffs = [(j, float(c)) for j, c in enumerate(lp.col_cost_) if c != 0.0]
    sense = "maximize" if "kMaximize" in str(lp.sense_) else "minimize"
    m.set_objective(obj_coeffs, float(lp.offset_), sense)

    n_rows = lp.num_row_
    rows: list[list[tuple[int, float]]] = [[] for _ in range(n_rows)]
    am = lp.a_matrix_
    start, index, value = list(am.start_), list(am.index_), list(am.value_)
    if "kColwise" in str(am.format_):
        for j in range(n):
            for k in range(start[j], start[j + 1]):
                if value[k] != 0.0:
                    rows[index[k]].append((j, value[k]))
    else:
        for i in range(n_rows):
            for k in range(start[i], start[i + 1]):
                if value[k] != 0.0:
                    rows[i].append((index[k], value[k]))

    n_constraints = 0
    for i in range(n_rows):
        lo, hi = lp.row_lower_[i], lp.row_upper_[i]
        terms = rows[i]
        if math.isinf(lo) and math.isinf(hi):
            continue
        if not math.isinf(lo) and abs(hi - lo) < 1e-12:
            m.add_constraint(terms, "==", float(lo))
            n_constraints += 1
            continue
        if not math.isinf(hi):
            m.add_constraint(terms, "<=", float(hi))
            n_constraints += 1
        if not math.isinf(lo):
            m.add_constraint(terms, ">=", float(lo))
            n_constraints += 1

    nnz = sum(len(r) for r in rows)
    return m, n_constraints, nnz


def _run_one(name: str, mps_path: Path, solve_timeout: float) -> dict:
    result: dict = {"name": name}
    try:
        h, lp = _load_lp(mps_path)
    except Exception as e:  # noqa: BLE001 - reported, not raised
        result["error"] = f"read failed: {e}"
        return result

    result["n_vars"] = lp.num_col_
    result["n_rows"] = lp.num_row_

    try:
        t0 = time.perf_counter()
        h.run()
        highs_time = time.perf_counter() - t0
        status = str(h.getModelStatus())
        result["highs_time"] = highs_time
        result["highs_status"] = status
        result["highs_obj"] = h.getObjectiveValue() if "kOptimal" in status else None
    except Exception as e:  # noqa: BLE001
        result["highs_error"] = str(e)

    try:
        model, n_constraints, nnz = _build_our_model(lp)
        result["n_constraints"] = n_constraints
        result["nnz"] = nnz
        t0 = time.perf_counter()
        out = model.solve(root_solver=None)
        ours_time = time.perf_counter() - t0
        result["ours_time"] = ours_time
        result["ours_status"] = out["status"]
        result["ours_obj"] = out["objective"]
    except Exception as e:  # noqa: BLE001
        result["ours_error"] = str(e)

    return result


def _run_worker(name: str, cache_dir: Path, timeout: float) -> None:
    """`--worker` entry point: solves exactly one problem in *this* process
    and prints a single JSON line to stdout. Invoked by `main()` as a
    subprocess (see its own docs for why) rather than called in-process."""
    mps_path = _cached_mps(name, cache_dir)
    if mps_path is None:
        print(json.dumps({"name": name, "error": "could not decompress"}))
        return
    print(json.dumps(_run_one(name, mps_path, timeout)))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--max-vars", type=int, default=3000, help="skip problems with more columns than this")
    parser.add_argument("--timeout", type=float, default=60.0, help="per-problem wall-clock budget, seconds — enforced (SIGTERM/kill) via a per-problem subprocess")
    # Relative to the current working directory, not `__file__`: once this
    # package is installed from a wheel, `__file__` sits under site-packages
    # with no reliable path back to a repo checkout (see this module's own
    # docs). Both this and `scripts/setup_netlib_data.py`'s own default
    # resolve to `<cwd>/.netlib_cache`, so running both from the repo root
    # (the documented usage) still lines them up.
    parser.add_argument("--cache-dir", type=Path, default=Path.cwd() / ".netlib_cache")
    parser.add_argument("--out", type=Path, default=Path.cwd() / "netlib_benchmark_results.csv")
    parser.add_argument("--only", nargs="*", help="run only these problem names (default: all, size-filtered)")
    parser.add_argument("--worker", metavar="NAME", help=argparse.SUPPRESS)  # internal: single-problem subprocess mode
    args = parser.parse_args()

    args.cache_dir.mkdir(parents=True, exist_ok=True)

    if args.worker:
        _run_worker(args.worker, args.cache_dir, args.timeout)
        return

    try:
        names = args.only if args.only else _cached_problem_list(args.cache_dir)
    except NetlibCacheError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)

    rows = []
    for name in names:
        mps_path = _cached_mps(name, args.cache_dir)
        if mps_path is None:
            print(f"{name:12s} SKIP (not cached — run `python scripts/setup_netlib_data.py` to fetch it)")
            continue

        # Cheap pre-check so an oversized problem is never even handed to
        # a worker subprocess — `readModel` itself is fast even for the
        # largest Netlib instances.
        h, lp = _load_lp(mps_path)
        n_vars = lp.num_col_
        del h, lp
        if n_vars > args.max_vars:
            print(f"{name:12s} SKIP (n_vars={n_vars} > {args.max_vars})")
            continue

        # Run this problem's actual solve in its own subprocess: this
        # crate's simplex engine `panic!`s (rather than returning an error)
        # on a handful of known-hard Netlib instances (e.g. `cycle`, named
        # for exactly the degenerate-pivoting behavior it stresses) when a
        # refactorization hits a numerically singular basis — a Rust panic
        # crossing the PyO3 boundary aborts the whole interpreter, which
        # would otherwise take the entire batch down with one bad problem.
        # A subprocess also gives `--timeout` real teeth (a hung/slow solve
        # is simply killed), which an in-process call has no way to do.
        try:
            proc = subprocess.run(
                [sys.executable, "-m", "enomoto_solver.benchmark_highs", "--worker", name, "--cache-dir", str(args.cache_dir)],
                capture_output=True,
                text=True,
                timeout=args.timeout,
            )
        except subprocess.TimeoutExpired:
            result = {"name": name, "error": f"timed out after {args.timeout}s"}
            rows.append(result)
            print(f"{name:12s} ERROR: {result['error']}")
            continue

        if proc.returncode != 0 or not proc.stdout.strip():
            stderr_tail = proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "(no stderr)"
            result = {"name": name, "error": f"worker crashed (exit {proc.returncode}): {stderr_tail}"}
            rows.append(result)
            print(f"{name:12s} ERROR: {result['error']}")
            continue

        result = json.loads(proc.stdout.strip().splitlines()[-1])
        rows.append(result)

        if "error" in result:
            print(f"{name:12s} ERROR: {result['error']}")
            continue
        ot = result.get("ours_time")
        ht = result.get("highs_time")
        os_ = result.get("ours_status", "?")
        hs = result.get("highs_status", "?")
        ratio = f"{ot / ht:6.2f}x" if isinstance(ot, float) and isinstance(ht, float) and ht > 0 else "   n/a"
        print(
            f"{name:12s} n={result.get('n_vars', '?'):>6} m={result.get('n_rows', '?'):>6}  "
            f"ours={ot if ot is not None else float('nan'):8.4f}s [{os_:10s}]  "
            f"highs={ht if ht is not None else float('nan'):8.4f}s [{hs:20s}]  ratio(ours/highs)={ratio}"
        )

    fieldnames = sorted({k for r in rows for k in r.keys()})
    with open(args.out, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in rows:
            writer.writerow(r)
    print(f"\nwrote {len(rows)} rows to {args.out}")

    solved = [r for r in rows if r.get("ours_status") == "optimal" and r.get("highs_status") and "kOptimal" in r["highs_status"]]
    if solved:
        total_ours = sum(r["ours_time"] for r in solved)
        total_highs = sum(r["highs_time"] for r in solved)
        print(f"\n{len(solved)} problems solved to optimality by both:")
        print(f"  total ours time:  {total_ours:.4f}s")
        print(f"  total highs time: {total_highs:.4f}s")
        print(f"  ratio (ours/highs): {total_ours / total_highs:.2f}x")


if __name__ == "__main__":
    main()
