"""Constraint: Function をスカラーまたは Function と ``<=`` / ``>=`` / ``==`` の
いずれか 1 つで比較した結果。

    g = 0 <= f          # 反射により Function.__ge__(f, 0)  -> ">="
    g = f >= 3           # Function.__ge__(f, 3)             -> ">="
    g = f == 10          # Function.__eq__(f, 10)            -> "=="

Constraint は ``coeffs . x  {sense}  rhs`` の形に正規化されており、
``Model.add_constraint`` に渡すまでは何の効果も持たない。
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict

if TYPE_CHECKING:
    from .model import Model

# 使用できる比較演算子
_SENSES = ("<=", ">=", "==")


class Constraint:
    """制約 1 本: ``sum(coeffs[j] * x_j)  sense  rhs``。

    属性:
        model: この制約が属する Model。
        coeffs: 変数番号 -> 係数 の dict。
        sense: ``"<="`` / ``">="`` / ``"=="`` のいずれか。
        rhs: 右辺の定数 (float)。
    """

    __slots__ = ("model", "coeffs", "sense", "rhs")

    def __init__(self, model: "Model", coeffs: Dict[int, float], sense: str, rhs: float):
        """制約を作る。``sense`` が不正なら ValueError。``coeffs`` は複製して保持する。"""
        if sense not in _SENSES:
            raise ValueError(f"sense must be one of {_SENSES}, got {sense!r}")
        self.model = model
        self.coeffs = dict(coeffs)
        self.sense = sense
        self.rhs = float(rhs)

    def nonzero_terms(self):
        """係数が厳密に 0 でない ``(変数番号, 係数)`` の組のリスト (Rust コアへ渡す形)。"""
        return [(j, c) for j, c in self.coeffs.items() if c != 0.0]

    def __repr__(self) -> str:
        """``Constraint(1*x0 + 2*x1 <= 10)`` のような表示用文字列。"""
        terms = " + ".join(f"{c:g}*x{j}" for j, c in sorted(self.coeffs.items()) if c != 0.0)
        terms = terms or "0"
        return f"Constraint({terms} {self.sense} {self.rhs:g})"
