"""Model: 問題入力インターフェースの入口。

    M = Model()
    x = Variable(float, 0, 10)   # 現在の Model (M) に属する
    f = x + 2 * y
    M.set_objective(f)
    M.add_constraint(0 <= f)
    M.solve()

Model 自体は行列を持たず、変数・目的関数・制約の宣言を Rust コア (``self._core``) に
転送するだけ。行列の保持・前処理・最適化アルゴリズムは Rust コア側にある。
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, List, Optional

from . import _core
from .constraint import Constraint
from .function import Function


@dataclass(frozen=True)
class Solution:
    """``Model.solve()`` の結果。

    属性:
        status: ``"optimal"`` / ``"infeasible"`` / ``"unbounded"`` /
            ``"infeasible_or_unbounded"`` / ``"not_solved"`` のいずれか。
        objective: 最適値 (``"optimal"`` 以外では None)。
        node_limit_hit: 整数計画でノード数上限により打ち切られたか (最適性は未証明)。
    """

    status: str
    objective: Optional[float]
    node_limit_hit: bool


class Model:
    """最適化モデル。``Model()`` を作るとそれが *現在の* Model になり、``model=`` を
    指定しない ``Variable(...)`` は、最後に作られた (または ``with model:`` で入った)
    Model に属する。明示したい場合は ``Variable(..., model=...)`` を使う。"""

    # 現在の Model のスタック (末尾が現在の Model)
    _stack: ClassVar[List["Model"]] = []

    def __init__(self):
        """空のモデルを作り、現在の Model にする。"""
        self._core = _core.PyModel()
        self._variables: List["Variable"] = []  # noqa: F821 - 循環 import を避けるため Variable は遅延 import
        self._objective: Optional[Function] = None
        self._constraints: List[Constraint] = []
        self._solution: Optional[Solution] = None
        Model._stack.append(self)

    @classmethod
    def current(cls) -> "Model":
        """現在の Model を返す。1 つもなければ RuntimeError。"""
        if not cls._stack:
            raise RuntimeError(
                "no active Model — create one with `M = Model()` before defining Variables, "
                "or pass model=<Model> explicitly to Variable(...)"
            )
        return cls._stack[-1]

    def __enter__(self) -> "Model":
        """``with model:`` の間、このモデルを現在の Model にする。"""
        Model._stack.append(self)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        """``with`` ブロックを抜けたら現在の Model を元に戻す。"""
        Model._stack.pop()

    def _register_variable(self, var) -> None:
        """Variable の生成時に呼ばれ、変数を Python 側の一覧に加える。"""
        self._variables.append(var)

    def _variable_value(self, index: int) -> float:
        """変数番号 ``index`` の最適解の値。最適解がなければ RuntimeError。"""
        if self._solution is None or self._solution.status != "optimal":
            raise RuntimeError("Model has not been solved to optimality yet — call Model.solve() first")
        return self._solution_x[index]

    # -- 問題の定義 --------------------------------------------------------
    def set_objective(self, f: Function, sense: str = "minimize") -> None:
        """目的関数 ``f`` と向き (``"minimize"`` / ``"maximize"``) を設定する
        (2 回目以降は上書き)。"""
        if not isinstance(f, Function):
            raise TypeError(f"Model.set_objective expects a Function, got {type(f).__name__}")
        if sense not in ("minimize", "maximize"):
            raise ValueError("sense must be 'minimize' or 'maximize'")
        self._objective = f
        self._core.set_objective(f.nonzero_terms(), f.constant, sense)

    def add_constraint(self, g: Constraint) -> None:
        """制約 ``g`` を追加する。"""
        if not isinstance(g, Constraint):
            raise TypeError(f"Model.add_constraint expects a Constraint, got {type(g).__name__}")
        self._constraints.append(g)
        self._core.add_constraint(g.nonzero_terms(), g.sense, g.rhs)

    # -- 求解 ------------------------------------------------------------
    def solve(
        self,
        root_solver: Optional[str] = None,
        distinguish_infeasible_unbounded: bool = False,
    ) -> Solution:
        """Rust コアで前処理と最適化を実行し、結果を Solution で返す。

        最適解が得られたかどうかにかかわらず例外は送出しない。結果は ``Solution.status``
        で判断する。``"optimal"`` のときだけ ``objective`` に値が入り、各 Variable の
        ``.value`` が読める (それ以外で ``.value`` を読むと RuntimeError)。

        ``root_solver``: 各 LP (整数計画では各緩和問題) の解法。

        - ``"auto"`` (既定、``None`` も同じ): 前処理後の行数が 5000 以上なら、傾き・切片双対二段解法と
          内点法 + クロスオーバーを別スレッドで同時に解き、先に結論を出した側を採る (もう一方は打ち切る)。
          未満なら傾き・切片双対二段解法だけで解く。実行不能・非有界の判定は常に二段解法が出す
          (内点法 + クロスオーバーが採られるのは、単体法で最適性を確かめた基底解が得られたときだけ)。
        - ``"simplex"``: 傾き・切片双対二段解法だけ。
        - ``"ipm_crossover"``: 内点法 + クロスオーバー (Liu & Lu 2024) だけ。内点法が収束しなければ
          二段解法で解き直す。
        - ``"interior"``: 前処理も独立の IP-PMM 内点法 (基底解ではない。突き合わせ検証用)。

        ``distinguish_infeasible_unbounded``: 単体法の結果状態は既定では ``"optimal"``、
        ``"infeasible"``、``"infeasible_or_unbounded"`` のいずれか。拡張双対単体法の
        段階 A が ``z^1 < 0`` で終われば有限の最適値がないことが証明されてそこで止まり、
        ``z^1 = 0`` なら非有界ではなく、段階 B で最適解を求めるか実行不能を証明する。
        True を渡すと前者も ``"infeasible"`` と ``"unbounded"`` に分ける。
        ``"not_solved"`` はソルバーが判定に至らずに諦めたことを表す。
        """
        result = self._core.solve(
            root_solver=root_solver,
            distinguish_infeasible_unbounded=distinguish_infeasible_unbounded,
        )
        status = result["status"]
        self._solution_x = result["x"]
        self._solution = Solution(
            status=status,
            objective=result["objective"],
            node_limit_hit=result["node_limit_hit"],
        )
        return self._solution

    def __repr__(self) -> str:
        """変数数・制約数・目的関数の設定有無を示す表示用文字列。"""
        return (
            f"Model(variables={self._core.n_variables()}, "
            f"constraints={self._core.n_constraints()}, "
            f"objective={'set' if self._objective is not None else 'unset'})"
        )
