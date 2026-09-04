import math

import pytest

from enomoto_solver import Constraint, Function, InfeasibleError, Model, Variable


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


def test_infinite_bounds_raise():
    M = Model()
    with pytest.raises(ValueError):
        Variable(float, 0, math.inf)
    with pytest.raises(ValueError):
        Variable(float, -math.inf, 0)
    with pytest.raises(ValueError):
        Variable(float, -math.inf, math.inf)


def test_binary_knapsack():
    M = Model()
    items = [(Variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
    M.set_objective(sum(v * val for v, val, _ in items), sense="maximize")
    M.add_constraint(sum(v * w for v, _, w in items) <= 50)
    sol = M.solve()
    assert approx(sol.objective, 220.0)


def test_infeasible_raises():
    M = Model()
    w = Variable(float, 0, 5)
    M.set_objective(w)
    M.add_constraint(w >= 10)
    with pytest.raises(InfeasibleError):
        M.solve()


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
