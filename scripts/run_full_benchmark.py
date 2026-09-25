"""Runs the full NETLIB-93 benchmark (this crate vs HiGHS) and writes
`scripts/benchmark_results.json` in the schema `docs/loop-design.md`'s
state-file rule expects (a `{"benchmark_config", "generated_at",
"problems"}` object, `problems` keyed by problem name), extended with the
per-problem LU-refactorization/FTRAN/BTRAN diagnostics introduced alongside
the deterministic-tick ("CLOCK") refactorization trigger — see
`analysis/ft_refactor_trigger_20260922_040850.md`.

New fields per problem (parsed from this crate's own
`ENOMOTO_PROF_PHASES_EXT` diagnostic, `src/simplex/slope_intercept_dual.rs`'s own
`prof_phases` module — collected in a *second*, untimed subprocess per
problem so the profiling overhead it adds (+5-8%, measured in the analysis
above) never contaminates `ours_time` itself, which stays comparable to
every earlier `benchmark_results.json`):
  ours_refactor_count        total LU refactorizations in the main loop.
  ours_refactor_count_clock  of which, caused by the new CLOCK trigger.
  ours_ftran_avg_us          average wall time of the entering-column FTRAN
                             (exactly one call per main-loop iteration).
  ours_btran_avg_us          average wall time of the rho_p BTRAN (exactly
                             one call per main-loop iteration).
  ours_iters                 main-loop iteration count (the denominator
                             above, and independently useful).

`highs_refactor_count` is obtained separately (an extra in-process HiGHS
run with `log_dev_level=2`): every `DuPh*`/`PrPh*` iteration-report line
HiGHS logs at that verbosity is emitted from exactly one `Ekk::rebuild()`
call (a fresh INVERT) — the same counting rule
`analysis/ft_refactor_trigger_20260922_040850.md` §7 describes by hand,
automated here. `highs_ftran_avg_us`/`highs_btran_avg_us` are intentionally
**not** collected: getting those needs HiGHS's own much heavier
`highs_analysis_level=63` timers (`SimplexInner-time`/`FactorLevel2-time`),
which both slow every solve down substantially and need per-phase log
parsing that was only ever run over the analysis's own 15-problem sample,
not designed to run unattended over all 93 problems on every benchmark
refresh — omitted rather than guessed at.

Usage: python scripts/run_full_benchmark.py [--out PATH] [--only NAME ...]
"""
from __future__ import annotations

import argparse
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

REBUILD_LINE_RE = re.compile(r"^(DuPh\d?|PrPh\d?)\s+\d+\s")
PROF_SUMMARY_RE = re.compile(
    r"PROF_PHASES_EXT wall=([\d.]+)ms iters=(\d+) .*refactor_count=(\d+) "
    r"\(verify=(\d+) try_update=(\d+) bump=(\d+) drift=(\d+) d_drift=(\d+) illcond=(\d+) "
    r"max_updates=(\d+) infeas_check=(\d+) clock=(\d+)\)"
)
PROF_PHASE_LINE_RE = re.compile(r"^\s+(\S+)\s+([\d.]+)ms\s+[\d.]+%\s+of wall\s+([\d.]+)us/iter")
PROF_POLISH_RE = re.compile(r"PROF_PHASES_EXT_POLISH refactor_count=(\d+) \(clock=(\d+)\)")


def _cached_problem_list(cache_dir: Path) -> list[str]:
    return [line.strip() for line in (cache_dir / "problems.txt").read_text().splitlines() if line.strip()]


def _highs_refactor_count(mps_path: Path) -> int | None:
    """Counts `Ekk::rebuild()` calls via a `log_dev_level=2` run — a
    separate, untimed HiGHS solve (never used for `highs_time`/`highs_obj`,
    which come from `_run_highs_timed` below's own plain run) so this
    extra logging overhead never contaminates the timing comparison
    `select_problem.py`/`docs/loop-design.md` depend on."""
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", True)
    h.setOptionValue("log_dev_level", 2)
    fd, tmp_path = tempfile.mkstemp()
    os.close(fd)
    try:
        sys.stdout.flush()
        old_fd = os.dup(1)
        try:
            with open(tmp_path, "w") as f:
                os.dup2(f.fileno(), 1)
                try:
                    status = h.readModel(str(mps_path))
                    if "kOk" in str(status):
                        h.run()
                finally:
                    sys.stdout.flush()
                    os.dup2(old_fd, 1)
        finally:
            os.close(old_fd)
        text = Path(tmp_path).read_text(errors="replace")
    finally:
        os.unlink(tmp_path)
    return sum(1 for line in text.splitlines() if REBUILD_LINE_RE.match(line))


def _run_highs_timed(mps_path: Path) -> dict:
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    status = h.readModel(str(mps_path))
    result: dict = {}
    if "kOk" not in str(status):
        result["highs_error"] = f"readModel failed: {status}"
        return result
    t0 = time.perf_counter()
    h.run()
    result["highs_time"] = time.perf_counter() - t0
    result["highs_status"] = str(h.getModelStatus())
    result["highs_obj"] = h.getObjectiveValue() if "kOptimal" in result["highs_status"] else None
    lp = h.getLp()
    result["n_vars"] = lp.num_col_
    result["n_rows"] = lp.num_row_
    return result


def _run_ours_worker(name: str, cache_dir: Path, timeout: float, extra_env: dict) -> tuple[dict, str]:
    env = os.environ.copy()
    env.update(extra_env)
    proc = subprocess.run(
        [sys.executable, "-m", "enomoto_solver.benchmark_highs", "--worker", name, "--cache-dir", str(cache_dir)],
        capture_output=True, text=True, timeout=timeout, env=env,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        stderr_tail = proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "(no stderr)"
        return {"error": f"worker crashed (exit {proc.returncode}): {stderr_tail}"}, proc.stderr
    return json.loads(proc.stdout.strip().splitlines()[-1]), proc.stderr


def _parse_prof_diag(stderr: str) -> dict:
    out: dict = {}
    m = PROF_SUMMARY_RE.search(stderr)
    if m:
        out["ours_iters"] = int(m.group(2))
        out["ours_refactor_count"] = int(m.group(3))
        out["ours_refactor_count_clock"] = int(m.group(12))
    for line in stderr.splitlines():
        pm = PROF_PHASE_LINE_RE.match(line)
        if not pm:
            continue
        phase, _ms, us_per_iter = pm.group(1), pm.group(2), pm.group(3)
        if phase == "ftran":
            out["ours_ftran_avg_us"] = float(us_per_iter)
        elif phase == "btran(rho_p)":
            out["ours_btran_avg_us"] = float(us_per_iter)
    pm2 = PROF_POLISH_RE.search(stderr)
    if pm2:
        out["ours_refactor_count_polish"] = int(pm2.group(1))
        out["ours_refactor_count_polish_clock"] = int(pm2.group(2))
    return out


def run_one(name: str, cache_dir: Path, timeout: float) -> dict:
    result: dict = {"name": name}
    mps_path = cache_dir / "mps" / f"{name}.mps"
    if not mps_path.exists() or mps_path.stat().st_size == 0:
        result["error"] = "could not decompress"
        return result

    try:
        result.update(_run_highs_timed(mps_path))
    except Exception as e:  # noqa: BLE001
        result["highs_error"] = str(e)

    try:
        hrc = _highs_refactor_count(mps_path)
        if hrc is not None:
            result["highs_refactor_count"] = hrc
    except Exception as e:  # noqa: BLE001
        result["highs_refactor_count_error"] = str(e)

    try:
        timed, _ = _run_ours_worker(name, cache_dir, timeout, {})
    except subprocess.TimeoutExpired:
        timed = {"error": f"timed out after {timeout}s"}
    for k, v in timed.items():
        if k == "name":
            continue
        result[k] = v

    if "error" not in timed and "ours_error" not in timed:
        try:
            _, stderr = _run_ours_worker(name, cache_dir, timeout, {"ENOMOTO_PROF_PHASES_EXT": "1"})
            result.update(_parse_prof_diag(stderr))
        except subprocess.TimeoutExpired:
            pass

    return result


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".netlib_cache")
    ap.add_argument("--out", type=Path, default=REPO_ROOT / "scripts" / "benchmark_results.json")
    ap.add_argument("--timeout", type=float, default=90.0)
    ap.add_argument("--only", nargs="*")
    args = ap.parse_args()

    names = args.only if args.only else _cached_problem_list(args.cache_dir)
    problems: dict[str, dict] = {}
    for name in names:
        t0 = time.time()
        r = run_one(name, args.cache_dir, args.timeout)
        r.pop("name", None)
        problems[name] = r
        ot = r.get("ours_time")
        ht = r.get("highs_time")
        ratio = f"{ot / ht:6.2f}x" if isinstance(ot, (int, float)) and isinstance(ht, (int, float)) and ht > 0 else "   n/a"
        print(f"{name:12s} ours={ot} highs={ht} ratio={ratio} refactor(ours/highs)={r.get('ours_refactor_count')}/{r.get('highs_refactor_count')}  [{time.time()-t0:.1f}s]")

    out = {
        "benchmark_config": {
            "max_vars": None,
            "source": "python scripts/run_full_benchmark.py (all cached Netlib problems, no size filter)",
            "timeout_s": args.timeout,
        },
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "problems": problems,
    }
    args.out.write_text(json.dumps(out, indent=2, sort_keys=True))
    print(f"\nwrote {len(problems)} problems to {args.out}")


if __name__ == "__main__":
    main()
