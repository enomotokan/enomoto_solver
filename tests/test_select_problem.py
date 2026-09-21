"""Tests for the loop's step-0 problem selection (scripts/select_problem.py).

The ordering these pin down is the whole point of the module: a wrong
answer outranks an inaccurate one, which outranks a slow one, no matter
how slow.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

from select_problem import ACCURACY, CORRECTNESS, RUNTIME, rank, select  # noqa: E402


def _optimal(name_time=1.0, highs_time=1.0, ours_obj=100.0, highs_obj=100.0):
    return {
        "ours_status": "optimal",
        "highs_status": "HighsModelStatus.kOptimal",
        "ours_obj": ours_obj,
        "highs_obj": highs_obj,
        "ours_time": name_time,
        "highs_time": highs_time,
    }


def _results(problems):
    return {"problems": problems}


def test_correctness_beats_accuracy_and_runtime():
    results = _results(
        {
            "slow": _optimal(name_time=1000.0, highs_time=0.001),
            "inaccurate": _optimal(ours_obj=100.1),
            "wrong": {
                "ours_status": "infeasible",
                "highs_status": "HighsModelStatus.kOptimal",
                "highs_obj": -1.0,
                "ours_time": 0.1,
                "highs_time": 0.1,
            },
        }
    )
    ranked = rank(results, obj_tol=1e-6)
    assert [c["name"] for c in ranked] == ["wrong", "inaccurate", "slow"]
    assert ranked[0]["tier"] == CORRECTNESS
    assert ranked[1]["tier"] == ACCURACY
    assert ranked[2]["tier"] == RUNTIME


def test_accuracy_ranked_by_relative_error():
    results = _results(
        {
            "small": _optimal(ours_obj=100.0 + 1e-4),
            "large": _optimal(ours_obj=100.0 + 1e-2),
            "exact": _optimal(),
        }
    )
    ranked = rank(results, obj_tol=1e-6)
    assert [c["name"] for c in ranked] == ["large", "small", "exact"]
    assert ranked[2]["tier"] == RUNTIME  # within tolerance -> judged on speed only


def test_runtime_ranked_by_ratio_with_timeout_worst():
    results = _results(
        {
            "fast": _optimal(name_time=0.1, highs_time=0.1),
            "slow": _optimal(name_time=5.0, highs_time=0.1),
            "timeout": {"error": "timed out after 60.0s"},
        }
    )
    ranked = rank(results, obj_tol=1e-6)
    assert [c["name"] for c in ranked] == ["timeout", "slow", "fast"]
    assert all(c["tier"] == RUNTIME for c in ranked)


def test_crash_is_correctness_but_below_a_wrong_answer():
    results = _results(
        {
            "crashed": {"error": "worker crashed (exit -6): panicked at 'singular basis'"},
            "wrong": {
                "ours_status": "infeasible",
                "highs_status": "HighsModelStatus.kOptimal",
                "highs_obj": -1.0,
            },
        }
    )
    ranked = rank(results, obj_tol=1e-6)
    assert [c["name"] for c in ranked] == ["wrong", "crashed"]
    assert all(c["tier"] == CORRECTNESS for c in ranked)


def test_undecompressable_problem_is_never_selected():
    results = _results({"broken": {"error": "could not decompress"}, "slow": _optimal(name_time=9.0, highs_time=0.1)})
    out = select(results, state={}, obj_tol=1e-6, max_failures=3)
    assert out["selected"]["name"] == "slow"
    assert [c["name"] for c in out["data_errors"]] == ["broken"]


def test_repeatedly_failed_problem_is_skipped():
    results = _results({"wrong": {"ours_status": "infeasible", "highs_status": "HighsModelStatus.kOptimal"}, "slow": _optimal(name_time=9.0, highs_time=0.1)})
    state = {"problems": {"wrong": {"consecutive_failures": 3}}}
    out = select(results, state, obj_tol=1e-6, max_failures=3)
    assert out["selected"]["name"] == "slow"
    assert out["exhausted"] is False


def test_all_problems_exhausted_falls_back_to_worst_and_flags_it():
    results = _results({"wrong": {"ours_status": "infeasible", "highs_status": "HighsModelStatus.kOptimal"}})
    state = {"problems": {"wrong": {"consecutive_failures": 3}}}
    out = select(results, state, obj_tol=1e-6, max_failures=3)
    assert out["selected"]["name"] == "wrong"
    assert out["exhausted"] is True


def test_real_baseline_ranks_tiers_in_order():
    """Against whatever baseline is committed: the pick is the top of the
    ranking, and the ranking never puts a slower-but-correct problem above
    a wrong or inaccurate one. Deliberately not pinned to a problem name —
    the baseline changes every time the loop improves something."""
    import json

    from select_problem import TIER_ORDER

    path = Path(__file__).resolve().parents[1] / "scripts" / "benchmark_results.json"
    if not path.exists():
        pytest.skip("no committed baseline")
    results = json.loads(path.read_text(encoding="utf-8"))
    out = select(results, state={}, obj_tol=1e-6, max_failures=3)
    ranked = out["ranked"]
    assert ranked, "baseline produced no actionable problems"
    assert out["selected"]["name"] == ranked[0]["name"]
    tiers = [TIER_ORDER[c["tier"]] for c in ranked]
    assert tiers == sorted(tiers)
