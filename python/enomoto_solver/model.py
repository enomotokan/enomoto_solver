"""Model: the entry point of the problem-input interface.

    M = Model()
    x = Variable(float, 0, 10)   # attaches to the current Model (M)
    f = x + 2 * y
    M.set_objective(f)
    M.add_constraint(0 <= f)
    M.solve()

Model itself holds no matrices — it only forwards variable/objective/
constraint declarations to the Rust core (``self._core``), which is where
the CSR storage, preprocessing and optimization algorithm live.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, List, Optional

from . import _core
from .constraint import Constraint
from .exceptions import InfeasibleError, UnboundedError
from .function import Function


@dataclass(frozen=True)
class Solution:
    status: str
    objective: Optional[float]
    node_limit_hit: bool


class Model:
    """Every ``Model()`` becomes the *current* model, so a bare
    ``Variable(...)`` call attaches to whichever Model was created (or
    entered via ``with model:``) most recently. Use ``model=`` explicitly
    on ``Variable(...)`` to opt out."""

    _stack: ClassVar[List["Model"]] = []

    def __init__(self):
        self._core = _core.PyModel()
        self._variables: List["Variable"] = []  # noqa: F821 - Variable imported lazily to avoid a cycle
        self._objective: Optional[Function] = None
        self._constraints: List[Constraint] = []
        self._solution: Optional[Solution] = None
        Model._stack.append(self)

    @classmethod
    def current(cls) -> "Model":
        if not cls._stack:
            raise RuntimeError(
                "no active Model — create one with `M = Model()` before defining Variables, "
                "or pass model=<Model> explicitly to Variable(...)"
            )
        return cls._stack[-1]

    def __enter__(self) -> "Model":
        Model._stack.append(self)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        Model._stack.pop()

    def _register_variable(self, var) -> None:
        self._variables.append(var)

    def _variable_value(self, index: int) -> float:
        if self._solution is None or self._solution.status != "optimal":
            raise RuntimeError("Model has not been solved to optimality yet — call Model.solve() first")
        return self._solution_x[index]

    # -- problem definition ------------------------------------------------
    def set_objective(self, f: Function, sense: str = "minimize") -> None:
        if not isinstance(f, Function):
            raise TypeError(f"Model.set_objective expects a Function, got {type(f).__name__}")
        if sense not in ("minimize", "maximize"):
            raise ValueError("sense must be 'minimize' or 'maximize'")
        self._objective = f
        self._core.set_objective(f.nonzero_terms(), f.constant, sense)

    def add_constraint(self, g: Constraint) -> None:
        if not isinstance(g, Constraint):
            raise TypeError(f"Model.add_constraint expects a Constraint, got {type(g).__name__}")
        self._constraints.append(g)
        self._core.add_constraint(g.nonzero_terms(), g.sense, g.rhs)

    # -- solve ------------------------------------------------------------
    def solve(self, raise_on_failure: bool = True) -> Solution:
        """Runs preprocessing + the optimization algorithm in the Rust
        core. On success, each Variable's ``.value`` becomes readable."""
        result = self._core.solve()
        status = result["status"]
        self._solution_x = result["x"]
        self._solution = Solution(
            status=status,
            objective=result["objective"],
            node_limit_hit=result["node_limit_hit"],
        )

        if raise_on_failure and status == "infeasible":
            raise InfeasibleError("model is infeasible: no assignment satisfies all constraints")
        if raise_on_failure and status == "unbounded":
            raise UnboundedError("objective is unbounded on the feasible region")

        return self._solution

    def __repr__(self) -> str:
        return (
            f"Model(variables={self._core.n_variables()}, "
            f"constraints={self._core.n_constraints()}, "
            f"objective={'set' if self._objective is not None else 'unset'})"
        )
