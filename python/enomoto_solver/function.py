"""Function: a linear expression over a Model's variables.

Internally a Function is nothing more than a sparse coefficient map
``{variable_index: coefficient}`` plus a constant term — the Python-side
"problem input interface" never holds the constraint/objective matrices
itself; those live in the Rust core as CSR matrices. Function only carries
enough information to describe *one row* (or the objective) until it is
handed to ``Model.set_objective`` / ``Model.add_constraint``.
"""

from __future__ import annotations

from numbers import Real
from typing import TYPE_CHECKING, Dict

if TYPE_CHECKING:
    from .model import Model
    from .constraint import Constraint

Number = Real


class Function:
    __slots__ = ("model", "coeffs", "constant")

    def __init__(self, model: "Model", coeffs: Dict[int, float] | None = None, constant: float = 0.0):
        self.model = model
        self.coeffs: Dict[int, float] = dict(coeffs) if coeffs else {}
        self.constant = float(constant)

    # -- helpers -----------------------------------------------------
    def _same_model(self, other: "Function") -> None:
        if self.model is not other.model:
            raise ValueError("cannot combine Function/Variable objects that belong to different Models")

    def _coerce(self, other) -> "Function | None":
        if isinstance(other, Function):
            self._same_model(other)
            return other
        if isinstance(other, Number) and not isinstance(other, bool):
            return Function(self.model, {}, float(other))
        return None

    def nonzero_terms(self):
        """Sparse (index, coeff) pairs, dropping exact zeros."""
        return [(j, c) for j, c in self.coeffs.items() if c != 0.0]

    # -- arithmetic ----------------------------------------------------
    def __add__(self, other):
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        coeffs = dict(self.coeffs)
        for j, c in rhs.coeffs.items():
            coeffs[j] = coeffs.get(j, 0.0) + c
        return Function(self.model, coeffs, self.constant + rhs.constant)

    __radd__ = __add__

    def __neg__(self):
        return Function(self.model, {j: -c for j, c in self.coeffs.items()}, -self.constant)

    def __sub__(self, other):
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        return self + (-rhs)

    def __rsub__(self, other):
        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        return (-self) + rhs

    def __mul__(self, other):
        if not isinstance(other, Number) or isinstance(other, bool):
            return NotImplemented
        scalar = float(other)
        return Function(self.model, {j: c * scalar for j, c in self.coeffs.items()}, self.constant * scalar)

    __rmul__ = __mul__

    # -- comparisons: build Constraint objects ---------------------------
    def __le__(self, other) -> "Constraint":
        return self._make_constraint(other, "<=")

    def __ge__(self, other) -> "Constraint":
        return self._make_constraint(other, ">=")

    def __eq__(self, other) -> "Constraint":  # type: ignore[override]
        return self._make_constraint(other, "==")

    __hash__ = None  # Function defines __eq__ for constraint-building, not value equality

    def _make_constraint(self, other, sense: str) -> "Constraint":
        from .constraint import Constraint

        rhs = self._coerce(other)
        if rhs is None:
            return NotImplemented
        diff = self - rhs  # Function: diff.coeffs . x + diff.constant  {sense}  0
        coeffs = {j: c for j, c in diff.coeffs.items() if c != 0.0}
        return Constraint(self.model, coeffs, sense, -diff.constant)

    def __repr__(self) -> str:
        terms = " + ".join(f"{c:g}*x{j}" for j, c in sorted(self.coeffs.items()) if c != 0.0)
        terms = terms or "0"
        if self.constant:
            terms += f" + {self.constant:g}"
        return f"Function({terms})"
