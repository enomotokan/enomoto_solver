"""Variable: Model に属する決定変数。

    M = Model()
    x = M.add_variable(float, 0, 10)   # 連続変数
    n = M.add_variable(int, 0, 10)     # 整数変数
    b = M.add_variable(int, 0, 1)      # 二値変数 == 境界 [0, 1] の整数変数

変数は ``Model.add_variable`` で作る (``Variable(...)`` を直接呼ぶと TypeError)。どのモデルに
入るかをコードの上で明らかにし、暗黙の「現在の Model」を持たないためである。

第 1 引数は変数の型で、組み込み型で指定する (``float`` -> 連続、``int`` -> 整数)。
文字列 ``"continuous"`` / ``"integer"`` も受け付ける。二値専用の型はない。

変数を作るとすぐに Rust コアに登録され (``model._core.add_variable``)、
その変数番号 (列番号) が返される。Variable 自身も Function (自分自身の 1 倍という
線形式) なので、Function の算術・比較演算がそのまま使える。
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Optional

from .function import Function

if TYPE_CHECKING:
    from .model import Model


class Variable(Function):
    """決定変数。``Model.add_variable`` (または ``Model.read_model``) が作る。

    属性:
        vtype: 正規化した型 (``"continuous"`` / ``"integer"``)。
        lb, ub: 下限・上限 (float。``±inf`` 可)。
        index: Rust コアでの変数番号。
        name: 表示名 (省略時は ``x{index}``)。
    """

    __slots__ = ("vtype", "lb", "ub", "index", "name")

    def __init__(self, *args, **kwargs):
        """直接は作れない。``M.add_variable(型, 下限, 上限)`` を使う。"""
        raise TypeError(
            "Variable(...) cannot be created directly; use M.add_variable(vtype, lb, ub) "
            "on the Model the variable belongs to (e.g. x = M.add_variable(float, 0, 10))"
        )

    @classmethod
    def _attach(cls, model: "Model", index: int, vtype: str, lb: float, ub: float, name: Optional[str]) -> "Variable":
        """Rust コアに登録済みの列 ``index`` を指す Variable を作る (``Model`` だけが使う)。"""
        v = cls.__new__(cls)
        Function.__init__(v, model, {index: 1.0}, 0.0)
        v.vtype = vtype
        v.lb = float(lb)
        v.ub = float(ub)
        v.index = index
        v.name = name or f"x{index}"
        return v

    # Function の += / -= はその場で書き換えるが、変数そのものを書き換えてはいけないので新しい式を返す
    def __iadd__(self, other):
        """``x += other``: 変数は書き換えず、新しい Function を返す。"""
        return self + other

    def __isub__(self, other):
        """``x -= other``: 変数は書き換えず、新しい Function を返す。"""
        return self - other

    @property
    def value(self) -> float:
        """直近の ``Model.solve()`` でこの変数に割り当てられた値。
        最適解が得られていなければ RuntimeError。"""
        return self.model._variable_value(self.index)

    def __repr__(self) -> str:
        """表示用文字列。"""
        return f"Variable(name={self.name!r}, vtype={self.vtype!r}, lb={self.lb:g}, ub={self.ub:g})"
