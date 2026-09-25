import math

import pytest

from enomoto_solver import (
    Constraint,
    Function,
    Model,
    Variable,
)


def approx(a, b, tol=1e-6):
    return abs(a - b) < tol


def test_lp_maximize_with_reflected_comparisons():
    M = Model()
    x = Variable(float, 0, 10)
    y = Variable(float, 0, 10)
    f = x + 2 * y
    assert isinstance(f, Function)
    M.set_objective(f, sense="maximize")
    M.add_constraint(x + y <= 10)
    M.add_constraint(0 <= x)  # reflected: int.__le__ -> NotImplemented -> Function.__ge__
    M.add_constraint(y >= 0)
    sol = M.solve()
    assert sol.status == "optimal"
    assert approx(sol.objective, 20.0)
    assert approx(x.value, 0.0)
    assert approx(y.value, 10.0)


def test_lp_minimize_with_ge_and_eq():
    M = Model()
    a = Variable(float, 0, 1000)
    b = Variable(float, 0, 1000)
    M.set_objective(a + b)
    M.add_constraint(a + 2 * b >= 6)
    M.add_constraint(a - b == 0)
    sol = M.solve()
    assert approx(sol.objective, 4.0)
    assert approx(a.value, 2.0)
    assert approx(b.value, 2.0)


def test_wide_but_finite_bounds():
    M = Model()
    z = Variable(float, -1_000_000, 1_000_000)
    M.set_objective(z)
    M.add_constraint(z >= -5)
    M.add_constraint(z <= 100)
    sol = M.solve()
    assert approx(sol.objective, -5.0)


def test_infinite_bounds_accepted():
    # `model.rs::add_variable`'s own docs: infinite lb/ub are a genuine
    # free or one-sided-unbounded variable (e.g. straight from an MPS
    # `FR`/`MI`/`PL` bound) and are no longer rejected here — only NaN and
    # lb > ub are.
    M = Model()
    a = Variable(float, 0, math.inf)
    b = Variable(float, -math.inf, 0)
    c = Variable(float, -math.inf, math.inf)
    assert (a.lb, a.ub) == (0, math.inf)
    assert (b.lb, b.ub) == (-math.inf, 0)
    assert (c.lb, c.ub) == (-math.inf, math.inf)


def test_nan_or_inverted_bounds_still_raise():
    M = Model()
    with pytest.raises(ValueError):
        Variable(float, 0, math.nan)
    with pytest.raises(ValueError):
        Variable(float, 10, 0)


def test_binary_knapsack():
    M = Model()
    items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
    M.set_objective(sum(v * val for v, val, _ in items), sense="maximize")
    M.add_constraint(sum(v * w for v, _, w in items) <= 50)
    sol = M.solve()
    assert approx(sol.objective, 220.0)


def test_infeasible_status():
    M = Model()
    w = Variable(float, 0, 5)
    M.set_objective(w)
    M.add_constraint(w >= 10)
    sol = M.solve()
    assert sol.status == "infeasible"
    assert sol.objective is None
    with pytest.raises(RuntimeError):
        w.value


def test_set_objective_type_error():
    M = Model()
    with pytest.raises(TypeError):
        M.set_objective(123)


def test_add_constraint_type_error():
    M = Model()
    with pytest.raises(TypeError):
        M.add_constraint(123)


def test_variables_from_different_models_cannot_combine():
    M1 = Model()
    M2 = Model()
    x1 = Variable(float, 0, 1, model=M1)
    x2 = Variable(float, 0, 1, model=M2)
    with pytest.raises(ValueError):
        _ = x1 + x2


def test_string_vtype_aliases_still_accepted():
    M = Model()
    x = Variable("continuous", 0, 1)
    n = Variable("integer", 0, 1)
    assert x.vtype == "continuous"
    assert n.vtype == "integer"


def test_unknown_vtype_raises():
    M = Model()
    with pytest.raises(ValueError):
        Variable(str, 0, 1)
    with pytest.raises(ValueError):
        Variable("nope", 0, 1)
    with pytest.raises(TypeError):
        Variable(3.14, 0, 1)


# -- stage-A early exit (z^1 < 0 => infeasible or unbounded) ----------------
# Both models below get past presolve with columns parked on an artificial
# `M` side, so the extended dual simplex's stage A ends with `z^1 < 0`.


def _unbounded_model():
    M = Model()
    x = [Variable(float, 0, math.inf, model=M) for _ in range(4)]
    M.set_objective(-x[0] - x[1] + x[2])
    M.add_constraint(x[0] - x[1] + x[2] + x[3] == 1)
    M.add_constraint(2 * x[0] - x[1] - x[2] <= 3)
    M.add_constraint(x[0] + x[2] >= 0.5)
    return M


def _infeasible_z1_negative_model():
    M = Model()
    x = [Variable(float, 0, math.inf, model=M) for _ in range(4)]
    M.set_objective(-x[0] - x[1])
    M.add_constraint(x[0] - x[1] + x[2] - x[3] == 1)
    M.add_constraint(x[0] - x[1] + x[2] - x[3] == 2)
    M.add_constraint(x[0] + x[1] - x[2] >= 0.5)
    return M


@pytest.mark.parametrize("build", [_unbounded_model, _infeasible_z1_negative_model])
def test_no_finite_optimum_reported_early_by_default(build):
    sol = build().solve()
    assert sol.status == "infeasible_or_unbounded"
    assert sol.objective is None


def test_distinguish_infeasible_unbounded_runs_stage_b():
    sol = _unbounded_model().solve(distinguish_infeasible_unbounded=True)
    assert sol.status == "unbounded"
    assert sol.objective is None
    sol = _infeasible_z1_negative_model().solve(distinguish_infeasible_unbounded=True)
    assert sol.status == "infeasible"
    assert sol.objective is None
