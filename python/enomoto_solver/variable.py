"""Variable: Model に属する決定変数。

    x = Variable(float, 0, 10)   # 連続変数
    n = Variable(int, 0, 10)     # 整数変数
    b = Variable(int, 0, 1)      # 二値変数 == 境界 [0, 1] の整数変数

第 1 引数は変数の型で、組み込み型で指定する (``float`` -> 連続、``int`` -> 整数)。
文字列 ``"continuous"`` / ``"integer"`` も受け付ける。二値専用の型はない。

Variable を作るとすぐに Rust コアに登録され (``model._core.add_variable``)、
その変数番号 (列番号) が返される。Variable 自身も Function (自分自身の 1 倍という
線形式) なので、Function の算術・比較演算がそのまま使える。
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Optional, Union

from .function import Function
from .types import normalize_vtype

if TYPE_CHECKING:
    from .model import Model


class Variable(Function):
    """決定変数。

    属性:
        vtype: 正規化した型 (``"continuous"`` / ``"integer"``)。
        lb, ub: 下限・上限 (float。``±inf`` 可)。
        index: Rust コアでの変数番号。
        name: 表示名 (省略時は ``x{index}``)。
    """

    __slots__ = ("vtype", "lb", "ub", "index", "name")

    def __init__(
        self,
        vtype: Union[type, str],
        lb: float,
        ub: float,
        *,
        model: Optional["Model"] = None,
        name: Optional[str] = None,
    ):
        """変数を作って ``model`` (省略時は現在の Model) に登録する。"""
        from .model import Model as _Model

        model = model or _Model.current()
        norm_vtype = normalize_vtype(vtype)
        index = model._core.add_variable(norm_vtype, float(lb), float(ub))

        super().__init__(model, {index: 1.0}, 0.0)
        self.vtype = norm_vtype
        self.lb = float(lb)
        self.ub = float(ub)
        self.index = index
        self.name = name or f"x{index}"
        model._register_variable(self)

    @property
    def value(self) -> float:
        """直近の ``Model.solve()`` でこの変数に割り当てられた値。
        最適解が得られていなければ RuntimeError。"""
        return self.model._variable_value(self.index)

    def __repr__(self) -> str:
        """表示用文字列。"""
        return f"Variable(name={self.name!r}, vtype={self.vtype!r}, lb={self.lb:g}, ub={self.ub:g})"
