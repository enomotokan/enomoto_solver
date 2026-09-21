"""Step 0 of the self-refiring optimization loop: pick the one problem the
next iteration should work on, from the benchmark baseline on `main`.

The loop's ordering rule (see docs/loop-design.md) is, worst first:

1. `correctness` — this crate answers a *qualitatively different* question
   than HiGHS does (claims infeasible/unbounded where HiGHS proves an
   optimum, or vice versa), or the solve dies outright (panic/crash). A
   wrong answer beats every slow answer, however slow.
2. `accuracy` — both solvers reach an optimum but disagree about the
   objective value by more than `--obj-tol` (relative). Ranked by how
   large that disagreement is.
3. `runtime` — everything that is correct: ranked by `ours_time /
   highs_time`, with a timed-out solve treated as infinitely slow.

Problems whose `consecutive_failures` in `loop_state.json` has reached
`--max-consecutive-failures` are skipped, so the loop stops re-picking a
problem it has repeatedly failed to improve. If that leaves nothing, the
overall worst problem is returned anyway and `exhausted` is set, which the
caller should report rather than silently treat as a normal pick.

Entries that can't be acted on at all (the `.mps` never decompressed, or
HiGHS itself failed to read it) are reported separately under `data` and
never selected — they're a data-pipeline problem, not a solver problem.

Usage:
    python scripts/select_problem.py [--json]
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

CORRECTNESS, ACCURACY, RUNTIME, DATA = "correctness", "accuracy", "runtime", "data"

# Tier order used as the primary sort key: lower sorts first (= worse).
TIER_ORDER = {CORRECTNESS: 0, ACCURACY: 1, RUNTIME: 2}

# Within `correctness`, which flavor of wrongness is worst. A confident
# wrong *answer* (infeasible/unbounded claimed against a proven optimum)
# outranks a crash, because a crash at least doesn't lie to the caller.
_CORRECTNESS_KIND_ORDER = {
    "false_infeasible_or_unbounded": 0,
    "false_optimal": 1,
    "status_mismatch": 2,
    "crash": 3,
}


def _highs_optimal(entry: dict) -> bool:
    return "kOptimal" in (entry.get("highs_status") or "")


def _rel_obj_error(entry: dict) -> float | None:
    ours, highs = entry.get("ours_obj"), entry.get("highs_obj")
    if ours is None or highs is None:
        return None
    return abs(ours - highs) / max(abs(highs), 1.0)


def classify(name: str, entry: dict, obj_tol: float) -> dict:
    """Classifies one `benchmark_results.json` entry into a tier plus a
    severity that ranks it against the other entries in the same tier."""
    error = entry.get("error")
    if error:
        if "timed out" in error:
            # Not an answer at all, but the failure mode is "too slow":
            # ranked at the top of the runtime tier, never above a wrong
            # answer.
            return {
                "name": name,
                "tier": RUNTIME,
                "severity": math.inf,
                "reason": f"{error} (no answer within the per-problem budget)",
            }
        if "crashed" in error:
            return {
                "name": name,
                "tier": CORRECTNESS,
                "kind": "crash",
                "severity": 0.0,
                "reason": f"solver process died: {error}",
            }
        # `could not decompress` / `read failed`: nothing the solver can
        # be blamed for, and nothing it can fix.
        return {"name": name, "tier": DATA, "severity": 0.0, "reason": error}

    ours_status = entry.get("ours_status")
    highs_ok = _highs_optimal(entry)

    if highs_ok and ours_status in {"infeasible", "unbounded"}:
        return {
            "name": name,
            "tier": CORRECTNESS,
            "kind": "false_infeasible_or_unbounded",
            "severity": 0.0,
            "reason": f"ours={ours_status} but HiGHS proves an optimum ({entry.get('highs_obj')})",
        }
    if not highs_ok and ours_status == "optimal":
        return {
            "name": name,
            "tier": CORRECTNESS,
            "kind": "false_optimal",
            "severity": 0.0,
            "reason": f"ours=optimal but HiGHS reports {entry.get('highs_status')}",
        }
    if ours_status != "optimal" or not highs_ok:
        return {
            "name": name,
            "tier": CORRECTNESS,
            "kind": "status_mismatch",
            "severity": 0.0,
            "reason": f"ours={ours_status}, highs={entry.get('highs_status')}",
        }

    rel = _rel_obj_error(entry)
    if rel is not None and rel > obj_tol:
        return {
            "name": name,
            "tier": ACCURACY,
            "severity": rel,
            "reason": f"objective disagrees with HiGHS by {rel:.2e} (relative, tol {obj_tol:.0e})",
        }

    ours_t, highs_t = entry.get("ours_time"), entry.get("highs_time")
    ratio = ours_t / highs_t if ours_t is not None and highs_t else math.inf
    return {
        "name": name,
        "tier": RUNTIME,
        "severity": ratio,
        "reason": f"ours {ours_t:.4f}s vs highs {highs_t:.4f}s (ratio {ratio:.2f}x)"
        if ours_t is not None and highs_t
        else "no usable timings",
    }


def rank(results: dict, obj_tol: float) -> list[dict]:
    """All actionable problems, worst first."""
    ranked = [classify(name, entry, obj_tol) for name, entry in results.get("problems", {}).items()]
    ranked = [c for c in ranked if c["tier"] != DATA]
    ranked.sort(
        key=lambda c: (
            TIER_ORDER[c["tier"]],
            _CORRECTNESS_KIND_ORDER.get(c.get("kind", ""), 0) if c["tier"] == CORRECTNESS else 0,
            -c["severity"],
            c["name"],
        )
    )
    return ranked


def select(results: dict, state: dict, obj_tol: float, max_failures: int) -> dict:
    """Returns `{"selected": <candidate or None>, "exhausted": bool,
    "ranked": [...], "data_errors": [...]}`."""
    ranked = rank(results, obj_tol)
    failures = {name: p.get("consecutive_failures", 0) for name, p in state.get("problems", {}).items()}
    eligible = [c for c in ranked if failures.get(c["name"], 0) < max_failures]

    selected = eligible[0] if eligible else (ranked[0] if ranked else None)
    if selected is not None:
        selected = dict(selected, consecutive_failures=failures.get(selected["name"], 0))
    return {
        "selected": selected,
        "exhausted": not eligible and bool(ranked),
        "ranked": ranked,
        "data_errors": [
            classify(n, e, obj_tol)
            for n, e in results.get("problems", {}).items()
            if classify(n, e, obj_tol)["tier"] == DATA
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--results", type=Path, default=REPO_ROOT / "scripts" / "benchmark_results.json")
    parser.add_argument("--state", type=Path, default=REPO_ROOT / "scripts" / "loop_state.json")
    parser.add_argument("--obj-tol", type=float, default=1e-6, help="relative objective agreement required (default 1e-6)")
    parser.add_argument("--max-consecutive-failures", type=int, default=3)
    parser.add_argument("--top", type=int, default=10, help="how many ranked candidates to show")
    parser.add_argument("--json", action="store_true", help="emit the full ranking as JSON")
    args = parser.parse_args()

    results = json.loads(args.results.read_text(encoding="utf-8"))
    state = json.loads(args.state.read_text(encoding="utf-8")) if args.state.exists() else {}
    out = select(results, state, args.obj_tol, args.max_consecutive_failures)

    if args.json:
        print(json.dumps(out, indent=2, sort_keys=True))
        return

    sel = out["selected"]
    if sel is None:
        print("no actionable problem found (empty or unusable benchmark results)")
        return
    if out["exhausted"]:
        print(
            f"WARNING: every problem has reached {args.max_consecutive_failures} consecutive failures — "
            "falling back to the overall worst"
        )
    print(f"selected: {sel['name']}  [{sel['tier']}]  {sel['reason']}")
    print(f"  consecutive_failures so far: {sel['consecutive_failures']}")
    print(f"\ntop {args.top} candidates (worst first):")
    for c in out["ranked"][: args.top]:
        print(f"  {c['name']:12s} {c['tier']:12s} {c['reason']}")
    if out["data_errors"]:
        print("\nnot selectable (data/pipeline errors, not solver bugs):")
        for c in out["data_errors"]:
            print(f"  {c['name']:12s} {c['reason']}")


if __name__ == "__main__":
    main()
