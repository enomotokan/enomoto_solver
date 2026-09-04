"""ENOMOTO-Solver: a Python problem-input interface backed by a Rust core
(CSR storage, preprocessing, optimization algorithms).

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
from .exceptions import InfeasibleError, SolverError, UnboundedError
from .function import Function
from .model import Model, Solution
from .variable import Variable

__all__ = [
    "Model",
    "Variable",
    "Function",
    "Constraint",
    "Solution",
    "SolverError",
    "InfeasibleError",
    "UnboundedError",
]

__version__ = "0.1.0"
