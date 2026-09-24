class SolverError(Exception):
    """Base class for errors raised while building or solving a model."""


class InfeasibleError(SolverError):
    """Raised by Model.solve() when the problem has no feasible solution."""


class UnboundedError(SolverError):
    """Raised by Model.solve() when the objective is unbounded."""


class InfeasibleOrUnboundedError(SolverError):
    """Raised by Model.solve() when the problem is proven to have no finite
    optimum but was not classified further (the default; pass
    ``distinguish_infeasible_unbounded=True`` to get InfeasibleError or
    UnboundedError instead). Deliberately not a subclass of either, since
    it may be either one."""


class NotSolvedError(SolverError):
    """Raised by Model.solve() when the solver gave up without reaching a
    verdict (e.g. a numerically singular basis it could not recover from,
    or its iteration budget running out). Nothing is claimed about the
    model itself."""
