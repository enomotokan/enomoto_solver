"""Model.solve(seed=...) の確かめ: 1 スレッドなら同じ種で同じ結果、種を変えると道筋 (ノード数) が変わりうる、引数の型と範囲。"""
import random

import pytest

from enomoto_solver import Model, Variable


def _multi_knapsack(rng, n=40, m=5):
    """0-1 の多次元ナップサック (乱数 rng で作る)。乱数 1 の問題は、1 スレッドで種によってノード数が変わる (種 0 で 29 ノード)。"""
    M = Model()
    xs = [Variable(int, 0, 1) for _ in range(n)]
    val = [rng.randint(10, 100) for _ in range(n)]
    M.set_objective(sum(v * x for v, x in zip(val, xs)), sense="maximize")
    for _ in range(m):
        w = [rng.randint(5, 60) for _ in range(n)]
        M.add_constraint(sum(a * x for a, x in zip(w, xs)) <= sum(w) // 3)
    return M


def _solve(seed):
    return _multi_knapsack(random.Random(1)).solve(threads=1, log=False, seed=seed)


@pytest.mark.parametrize("seed", [0, 1, 12345, 2**64 - 1])
def test_same_seed_single_thread_is_reproducible(seed):
    a, b = _solve(seed), _solve(seed)
    assert a.status == b.status == "optimal"
    assert a.objective == b.objective
    assert a.nodes == b.nodes


def test_seed_none_is_the_environment_default(monkeypatch):
    # 環境変数 ENOMOTO_T_MIP_SEED は最初に読んだ値を覚えるので、ここでは未設定 (既定 0) の前提で None と 0 を比べる
    monkeypatch.delenv("ENOMOTO_T_MIP_SEED", raising=False)
    a, b = _solve(None), _solve(0)
    assert a.objective == b.objective
    assert a.nodes == b.nodes


def test_different_seeds_can_change_the_search_path_but_not_the_answer():
    sols = [_solve(s) for s in range(8)]
    assert all(s.status == "optimal" for s in sols)
    assert len({s.objective for s in sols}) == 1
    # 種で木のヒューリスティクスの乱数が変わり、探索の道筋 (ノード数) が変わりうる
    assert len({s.nodes for s in sols}) >= 2


@pytest.mark.parametrize("seed", [-1, 2**64])
def test_seed_out_of_range_raises_value_error(seed):
    with pytest.raises(ValueError):
        _multi_knapsack(random.Random(1), n=5, m=1).solve(log=False, seed=seed)


@pytest.mark.parametrize("seed", [True, 1.0, "1"])
def test_seed_of_wrong_type_raises_type_error(seed):
    with pytest.raises(TypeError):
        _multi_knapsack(random.Random(1), n=5, m=1).solve(log=False, seed=seed)


def test_seed_on_lp_is_accepted_and_ignored():
    def build():
        M = Model()
        x, y = Variable(float, 0, 10), Variable(float, 0, 10)
        M.set_objective(3 * x + 2 * y, sense="maximize")
        M.add_constraint(x + y <= 4)
        M.add_constraint(x + 3 * y <= 6)
        return M

    a = build().solve(log=False, seed=7)
    b = build().solve(log=False)
    assert a.status == b.status == "optimal"
    assert abs(a.objective - b.objective) < 1e-9
