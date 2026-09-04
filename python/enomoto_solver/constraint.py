"""Constraint: the result of comparing a Function against a scalar/Function
with exactly one of ``<=``, ``>=`` or ``==``.

    g = 0 <= f          # Function.__ge__(f, 0) via reflection -> ">="
    g = f >= 3           # Function.__ge__(f, 3)               -> ">="
    g = f == 10          # Function.__eq__(f, 10)               -> "=="

A Constraint is normalized to ``coeffs . x  {sense}  rhs`` and is inert
until passed to ``Model.add_constraint``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict

if TYPE_CHECKING:
    from .model import Model

_SENSES = ("<=", ">=", "==")


class Constraint:
    __slots__ = ("model", "coeffs", "sense", "rhs")

    def __init__(self, model: "Model", coeffs: Dict[int, float], sense: str, rhs: float):
        if sense not in _SENSES:
            raise ValueError(f"sense must be one of {_SENSES}, got {sense!r}")
        self.model = model
        self.coeffs = dict(coeffs)
        self.sense = sense
        self.rhs = float(rhs)

    def nonzero_terms(self):
        return [(j, c) for j, c in self.coeffs.items() if c != 0.0]

    def __repr__(self) -> str:
        terms = " + ".join(f"{c:g}*x{j}" for j, c in sorted(self.coeffs.items()) if c != 0.0)
        terms = terms or "0"
        return f"Constraint({terms} {self.sense} {self.rhs:g})"
