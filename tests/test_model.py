import math

import pytest

from enomoto_solver import (
    Constraint,
    Function,
    Model,
    Variable,
    quicksum,
)


def approx(a, b, tol=1e-6):
    return abs(a - b) < tol


def test_lp_maximize_with_reflected_comparisons():
    M = Model()
    x = M.add_variable(float, 0, 10)
    y = M.add_variable(float, 0, 10)
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
    a = M.add_variable(float, 0, 1000)
    b = M.add_variable(float, 0, 1000)
    M.set_objective(a + b)
    M.add_constraint(a + 2 * b >= 6)
    M.add_constraint(a - b == 0)
    sol = M.solve()
    assert approx(sol.objective, 4.0)
    assert approx(a.value, 2.0)
    assert approx(b.value, 2.0)


def test_wide_but_finite_bounds():
    M = Model()
    z = M.add_variable(float, -1_000_000, 1_000_000)
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
    a = M.add_variable(float, 0, math.inf)
    b = M.add_variable(float, -math.inf, 0)
    c = M.add_variable(float, -math.inf, math.inf)
    assert (a.lb, a.ub) == (0, math.inf)
    assert (b.lb, b.ub) == (-math.inf, 0)
    assert (c.lb, c.ub) == (-math.inf, math.inf)


def test_nan_or_inverted_bounds_still_raise():
    M = Model()
    with pytest.raises(ValueError):
        M.add_variable(float, 0, math.nan)
    with pytest.raises(ValueError):
        M.add_variable(float, 10, 0)


def test_binary_knapsack():
    M = Model()
    items = [(M.add_variable(int, 0, 1), value, weight) for value, weight in [(60, 10), (100, 20), (120, 30)]]
    M.set_objective(sum(v * val for v, val, _ in items), sense="maximize")
    M.add_constraint(sum(v * w for v, _, w in items) <= 50)
    sol = M.solve()
    assert approx(sol.objective, 220.0)


@pytest.mark.parametrize("lp_solver", ["simplex", "ipm_crossover", "interior"])
def test_lp_solver_on_integer_model_warns(lp_solver):
    M = Model()
    x = M.add_variable(int, 0, 3)
    M.set_objective(x, sense="maximize")
    M.add_constraint(2 * x <= 5)
    with pytest.warns(UserWarning, match="no effect on a model with integer variables"):
        sol = M.solve(lp_solver=lp_solver, log=False)
    assert approx(sol.objective, 2.0)


@pytest.mark.parametrize("lp_solver", [None, "auto"])
def test_lp_solver_auto_on_integer_model_does_not_warn(lp_solver):
    import warnings as w

    M = Model()
    x = M.add_variable(int, 0, 3)
    M.set_objective(x, sense="maximize")
    M.add_constraint(2 * x <= 5)
    with w.catch_warnings():
        w.simplefilter("error")
        M.solve(lp_solver=lp_solver, log=False)


@pytest.mark.parametrize("lp_solver", ["simplex", "ipm_crossover", "interior", "auto"])
def test_lp_solver_on_lp(lp_solver):
    M = Model()
    x = M.add_variable(float, 0, 3)
    M.set_objective(x, sense="maximize")
    M.add_constraint(2 * x <= 5)
    sol = M.solve(lp_solver=lp_solver, log=False)
    assert approx(sol.objective, 2.5)


def test_root_solver_is_deprecated_alias_of_lp_solver():
    M = Model()
    x = M.add_variable(float, 0, 3)
    M.set_objective(x, sense="maximize")
    M.add_constraint(2 * x <= 5)
    with pytest.warns(DeprecationWarning, match="use lp_solver"):
        sol = M.solve(root_solver="simplex", log=False)
    assert approx(sol.objective, 2.5)
    with pytest.raises(TypeError, match="only one of lp_solver and root_solver"):
        M.solve(lp_solver="simplex", root_solver="simplex", log=False)


def test_unknown_lp_solver_raises():
    M = Model()
    x = M.add_variable(float, 0, 3)
    M.set_objective(x, sense="maximize")
    with pytest.raises(ValueError, match="unknown lp_solver"):
        M.solve(lp_solver="foo", log=False)


def test_infeasible_status():
    M = Model()
    w = M.add_variable(float, 0, 5)
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
    x1 = M1.add_variable(float, 0, 1)
    x2 = M2.add_variable(float, 0, 1)
    with pytest.raises(ValueError):
        _ = x1 + x2


def test_string_vtype_aliases_still_accepted():
    M = Model()
    x = M.add_variable("continuous", 0, 1)
    n = M.add_variable("integer", 0, 1)
    assert x.vtype == "continuous"
    assert n.vtype == "integer"


def test_unknown_vtype_raises():
    M = Model()
    with pytest.raises(ValueError):
        M.add_variable(str, 0, 1)
    with pytest.raises(ValueError):
        M.add_variable("nope", 0, 1)
    with pytest.raises(TypeError):
        M.add_variable(3.14, 0, 1)


# -- stage-A early exit (z^1 < 0 => infeasible or unbounded) ----------------
# Both models below get past presolve with columns parked on an artificial
# `M` side, so the extended dual simplex's stage A ends with `z^1 < 0`.


def _unbounded_model():
    M = Model()
    x = [M.add_variable(float, 0, math.inf) for _ in range(4)]
    M.set_objective(-x[0] - x[1] + x[2])
    M.add_constraint(x[0] - x[1] + x[2] + x[3] == 1)
    M.add_constraint(2 * x[0] - x[1] - x[2] <= 3)
    M.add_constraint(x[0] + x[2] >= 0.5)
    return M


def _infeasible_z1_negative_model():
    M = Model()
    x = [M.add_variable(float, 0, math.inf) for _ in range(4)]
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


def test_forcing_row_at_large_scale_is_not_reported_infeasible():
    # 作業 #9 (ken-18 の誤った infeasible と同じ系統): 等式行 sum a_i x_i = 3S で全列が上限 S/a_i に張り付く
    # (強制行) 問題。S = 1e8 ではスケール後の量が大きく、前処理の絶対 1e-9 の判定が丸め程度の差で
    # 実行不能と誤判定していた。
    S = 1e8
    a = [0.0188829, 0.0227359, 0.974715]
    M = Model()
    xs = [M.add_variable(float, 0, S / ai) for ai in a]
    w = M.add_variable(float, 0, S)
    M.set_objective(w)
    M.add_constraint(a[0] * xs[0] + a[1] * xs[1] + a[2] * xs[2] == 3 * S)
    M.add_constraint(a[0] * xs[0] + w == 1.5 * S)
    sol = M.solve()
    assert sol.status == "optimal"
    assert abs(sol.objective - 0.5 * S) <= 1e-7 * S


@pytest.mark.parametrize(
    "make",
    [
        lambda x, y: x * y,
        lambda x, y: (x + 1) * (y + 2),
        lambda x, y: x**2,
        lambda x, y: 1 / x,
        lambda x, y: x / y,
        lambda x, y: 2**x,
    ],
)
def test_nonlinear_expression_raises_with_explanation(make):
    # 二次以上の式・式での割り算は、線型式しか扱えないことを示す TypeError になる
    M = Model()
    x = M.add_variable(float, 0, 1)
    y = M.add_variable(float, 0, 1)
    with pytest.raises(TypeError, match="not linear"):
        make(x, y)


def test_division_by_number_is_linear():
    M = Model()
    x = M.add_variable(float, 0, 10)
    M.set_objective(x / 2, sense="maximize")
    sol = M.solve()
    assert sol.status == "optimal"
    assert approx(sol.objective, 5.0)


@pytest.mark.parametrize("parity, expected", [(1, "infeasible"), (0, "unbounded")])
@pytest.mark.parametrize("threads", [1, 4])
def test_mip_infeasible_or_unbounded_distinguished_by_integer_intercept_problem(parity, expected, threads):
    # 緩和問題は x の方向に非有界。y + z が奇数かつ y - z が偶数 (parity=1) なら整数解がなく実行不能、
    # parity=0 なら整数解があり非有界 (前処理では見抜けないので、切片問題を整数に制限した問題で決まる)
    def build():
        M = Model()
        x = M.add_variable(float, 0, math.inf)
        y = M.add_variable(int, -50, 50)
        z = M.add_variable(int, -50, 50)
        w = M.add_variable(int, -100, 100)
        v = M.add_variable(int, -100, 100)
        M.set_objective(-1 * x)
        M.add_constraint(y + z - 2 * w == parity)
        M.add_constraint(y - z - 2 * v == 0)
        M.add_constraint(x - y - z >= 0)
        return M

    # 既定 (区別を頼まない) でも、前処理 (HNF の前処理) や整数解と LP の非有界な向きから決まるときは区別した状態を返してよい
    assert build().solve(threads=threads).status in ("infeasible_or_unbounded", expected)
    sol = build().solve(distinguish_infeasible_unbounded=True, threads=threads)
    assert sol.status == expected
    assert sol.objective is None


def _knapsack():
    M = Model()
    items = [(M.add_variable(int, 0, 1), v, w) for v, w in [(60, 10), (100, 20), (120, 30)]]
    M.set_objective(sum(x * v for x, v, _ in items), sense="maximize")
    M.add_constraint(sum(x * w for x, _, w in items) <= 50)
    return M


def test_log_is_printed_by_default(capsys):
    # 既定では、作者・ライセンス・連絡先の案内、問題の大きさ、求解の要点、結果を標準出力に出す
    sol = _knapsack().solve()
    out = capsys.readouterr().out
    assert sol.status == "optimal"
    assert "MIT License" in out
    assert "Kan Enomoto" in out
    assert "https://enomoto-math-labo.com/en" in out
    assert "enomotokan@gmail.com" in out
    assert "Problem: 3 variables (3 integer), 1 constraints" in out
    assert "Root LP:" in out
    assert "Result: optimal" in out
    assert out.index("MIT License") < out.index("Root LP:")


@pytest.mark.parametrize("threads", [1, 4])
def test_log_false_prints_nothing(capsys, threads):
    sol = _knapsack().solve(log=False, threads=threads)
    assert sol.status == "optimal"
    captured = capsys.readouterr()
    assert captured.out == ""
    assert captured.err == ""


_SMALL_MPS = """\
NAME          SMALL
OBJSENSE
    MAX
ROWS
 N  obj
 L  c1
 G  c2
 E  c3
COLUMNS
    x         obj          1.0   c1           1.0
    x         c2           1.0
    MARKER    'MARKER'     'INTORG'
    y         obj          2.0   c1           1.0
    y         c3           1.0
    n         obj          1.0   c3           1.0
    MARKER    'MARKER'     'INTEND'
RHS
    rhs       obj         -5.0   c1           4.5
    rhs       c2           0.5   c3           3.0
RANGES
    rng       c1           2.0
BOUNDS
 UP bnd       x            4.0
 UP bnd       n            10
ENDATA
"""


def test_read_model_mps(tmp_path):
    """MPS を読む口 ``Model.readModel``。整数列の既定の二値・RANGES・目的の定数を確かめる。
    max x + 2y + n + 5,  2.5 <= x + y <= 4.5,  x >= 0.5,  y + n == 3,
    x in [0, 4], y 二値, n in [0, 10] 整数  ->  y + n = 3 は y=1,n=2 か y=0,n=3 で、
    x <= 4.5 - y。y=1: 3.5 + 2 + 2 + 5 = 12.5、y=0: 4 + 0 + 3 + 5 = 12 -> 12.5。"""
    import gzip

    path = tmp_path / "small.mps"
    path.write_text(_SMALL_MPS)
    M = Model()
    M.readModel(path)
    assert [v.name for v in M.variables] == ["x", "y", "n"]
    y = M.get_variable("y")
    assert y.vtype == "integer" and (y.lb, y.ub) == (0.0, 1.0)
    assert M.get_variable("n").ub == 10.0
    sol = M.solve(log=False)
    assert sol.status == "optimal"
    assert approx(sol.objective, 12.5)
    assert approx(M.get_variable("x").value, 3.5)

    # gzip 圧縮のファイルも同じに読める
    gz = tmp_path / "small.mps.gz"
    with gzip.open(gz, "wt") as fh:
        fh.write(_SMALL_MPS)
    M2 = Model()
    M2.read_model(str(gz))
    assert approx(M2.solve(log=False).objective, 12.5)


def test_read_model_needs_empty_model_and_rejects_quadratic(tmp_path):
    path = tmp_path / "small.mps"
    path.write_text(_SMALL_MPS)
    M = Model()
    M.add_variable(float, 0, 1)
    with pytest.raises(ValueError):
        M.readModel(path)
    q = tmp_path / "q.mps"
    q.write_text("NAME\nROWS\n N obj\nCOLUMNS\n x obj 1\nQUADOBJ\n x x 1\nENDATA\n")
    with pytest.raises(ValueError):
        Model().readModel(q)


# -- 変数の作り方 (M.add_variable) と quicksum ------------------------------


def test_variable_cannot_be_created_directly():
    # 変数は M.add_variable で作る。Variable(...) を直接呼ぶと、どのモデルに入るか分からないので TypeError
    with pytest.raises(TypeError, match="add_variable"):
        Variable(float, 0, 1)


def test_add_variable_belongs_to_its_model_and_has_name():
    M1 = Model()
    M2 = Model()
    x = M1.add_variable(float, 0, 1, name="x")
    y = M2.add_variable(int, 0, 3)
    assert x.model is M1 and y.model is M2
    assert x.name == "x" and y.name == "x0"
    assert isinstance(x, Variable) and M1.variables == [x] and M2.variables == [y]
    with pytest.raises(ValueError):
        M1.add_variable(float, 1, 0)


def test_models_are_freed_when_unreferenced():
    # 暗黙の「現在の Model」を持たないので、使い終わった Model は参照が無くなれば解放される
    import gc
    import weakref

    M = Model()
    M.add_variable(float, 0, 1)
    ref = weakref.ref(M)
    del M
    gc.collect()
    assert ref() is None


def test_iadd_updates_in_place_but_not_variables():
    M = Model()
    x = M.add_variable(float, 0, 1)
    y = M.add_variable(float, 0, 1)
    f = x + 0
    g = f
    f += y
    f -= 2
    assert g is f and f.coeffs == {x.index: 1.0, y.index: 1.0} and f.constant == -2.0
    z = x
    z += y  # 変数は書き換えない
    assert x.coeffs == {x.index: 1.0} and z.coeffs == {x.index: 1.0, y.index: 1.0}


def test_quicksum_matches_sum():
    M = Model()
    xs = [M.add_variable(int, 0, 1) for _ in range(50)]
    w = list(range(1, 51))
    a = quicksum(wi * xi for wi, xi in zip(w, xs))
    b = sum(wi * xi for wi, xi in zip(w, xs))
    assert a.coeffs == b.coeffs and a.constant == b.constant
    assert quicksum([xs[0], 3, xs[0]]).coeffs == {xs[0].index: 2.0}
    assert quicksum([xs[0], 3]).constant == 3.0
    assert quicksum([]) == 0.0
    M.set_objective(quicksum(xs), sense="maximize")
    M.add_constraint(quicksum(w[i] * xs[i] for i in range(50)) <= 100)
    assert M.solve(log=False).status == "optimal"
