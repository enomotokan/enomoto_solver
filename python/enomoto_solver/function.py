"""Function: Model の変数の線形式。

Function の中身は疎な係数の dict ``{変数番号: 係数}`` と定数項だけ。Python 側は
制約行列や目的関数の行列を持たず、それらは Rust コアが持つ。Function は
``Model.set_objective`` / ``Model.add_constraint`` に渡されるまで、
1 行分 (または目的関数) を表すだけの情報を運ぶ。
"""

from __future__ import annotations

from numbers import Real
from typing import TYPE_CHECKING, Dict

if TYPE_CHECKING:
    from .model import Model
    from .constraint import Constraint

# スカラーとして受け付ける数値型 (bool は除外して扱う)
Number = Real


class Function:
    """線形式 ``sum(coeffs[j] * x_j) + constant``。

    ``+``, ``-``, スカラー倍 ``*`` で新しい Function を作り、``<=`` / ``>=`` / ``==`` で
    Constraint を作る。

    属性:
        model: この式が属する Model。
        coeffs: 変数番号 -> 係数 の dict。
        constant: 定数項 (float)。
    """

    __slots__ = ("model", "coeffs", "constant")

    def __init__(self, model: "Model", coeffs: Dict[int, float] | None = None, constant: float = 0.0):
        """式を作る。``coeffs`` は複製して保持する (省略時は空)。"""
        self.model = model
        self.coeffs: Dict[int, float] = dict(coeffs) if coeffs else {}
        self.constant = float(constant)

    # -- 補助関数 -----------------------------------------------------
    def _same_model(self, other: "Function") -> None:
        """``other`` が同じ Model に属していなければ ValueError。"""
        if self.model is not other.model:
            raise ValueError("cannot combine Function/Variable objects that belong to different Models")

    def _coerce(self, other) -> "Function | None":
        """``other`` を Function に変換する。Function ならそのまま (同じ Model か検査)、
        数値 (bool 以外) なら定数の Function、それ以外は None (演算子は NotImplemented を返す)。"""
        if isinstance(other, Function):
            self._same_model(other)
            return other
        if isinstance(other, Number) and not isinstance(other, bool):
            return Function(self.model, {}, float(other))
        return None

    def nonzero_terms(self):
        """係数が厳密に 0 でない ``(変数番号, 係数)`` の組のリスト (Rust コアへ渡す形)。"""
        return [(j, c) for j, c in self.coeffs.items() if c != 0.0]

    # -- 算術演算 ----------------------------------------------------
    def __add__(self, other):
        """``self + other`` (Function または数値)。"""
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        coeffs = dict(self.coeffs)
        for j, c in rhs.coeffs.items():
            coeffs[j] = coeffs.get(j, 0.0) + c
        return Function(self.model, coeffs, self.constant + rhs.constant)

    __radd__ = __add__

    def __neg__(self):
        """``-self``。"""
        return Function(self.model, {j: -c for j, c in self.coeffs.items()}, -self.constant)

    def __sub__(self, other):
        """``self - other``。"""
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        return self + (-rhs)

    def __rsub__(self, other):
        """``other - self``。"""
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        return (-self) + rhs

    def __mul__(self, other):
        """スカラー倍 ``self * other`` (数値のみ。式どうしの積は不可)。"""
        if not isinstance(other, Number) or isinstance(other, bool):
            return NotImplemented
        scalar = float(other)
        return Function(self.model, {j: c * scalar for j, c in self.coeffs.items()}, self.constant * scalar)

    __rmul__ = __mul__

    # -- 比較演算: Constraint を作る ------------------------------------
    def __le__(self, other) -> "Constraint":
        """``self <= other`` の Constraint を作る。"""
        return self._make_constraint(other, "<=")

    def __ge__(self, other) -> "Constraint":
        """``self >= other`` の Constraint を作る。"""
        return self._make_constraint(other, ">=")

    def __eq__(self, other) -> "Constraint":  # type: ignore[override]
        """``self == other`` の Constraint を作る (値の等価比較ではない)。"""
        return self._make_constraint(other, "==")

    __hash__ = None  # __eq__ は制約を作るためのもので値の等価ではないので、ハッシュ不可にする

    def _make_constraint(self, other, sense: str) -> "Constraint":
        """``self  sense  other`` を ``coeffs . x  sense  rhs`` に正規化した Constraint を作る。"""
        from .constraint import Constraint

        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        diff = self - rhs  # diff.coeffs . x + diff.constant  {sense}  0
        coeffs = {j: c for j, c in diff.coeffs.items() if c != 0.0}
        return Constraint(self.model, coeffs, sense, -diff.constant)

    def __repr__(self) -> str:
        """``Function(1*x0 + 2*x1 + 3)`` のような表示用文字列。"""
        terms = " + ".join(f"{c:g}*x{j}" for j, c in sorted(self.coeffs.items()) if c != 0.0)
        terms = terms or "0"
        if self.constant:
            terms += f" + {self.constant:g}"
        return f"Function({terms})"
