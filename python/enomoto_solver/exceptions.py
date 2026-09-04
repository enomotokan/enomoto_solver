class SolverError(Exception):
    """Base class for errors raised while building or solving a model."""


class InfeasibleError(SolverError):
    """Raised by Model.solve() when the problem has no feasible solution."""


class UnboundedError(SolverError):
    """Raised by Model.solve() when the objective is unbounded."""
