"""ENOMOTO-Solver: 線形最適化・混合整数最適化ソルバーの Python 入力インターフェース。
行列の保持・前処理・最適化アルゴリズムは Rust コア (``enomoto_solver._core``) が担う。

    from enomoto_solver import Model, Variable

    M = Model()
    x = Variable(float, 0, 10)
    y = Variable(float, 0, 10)
    f = x + 2 * y
    M.set_objective(f, sense="maximize")
    M.add_constraint(x + y <= 10)
    M.add_constraint(0 <= x)
    solution = M.solve()
    print(solution.objective, x.value, y.value)
"""

from .constraint import Constraint
from .function import Function
from .model import Model, Solution
from .variable import Variable
from ._core import max_threads

# 公開 API
__all__ = [
    "max_threads",
    "Model",
    "Variable",
    "Function",
    "Constraint",
    "Solution",
]

# パッケージのバージョン
__version__ = "0.1.0"
