"""Variable: a decision variable belonging to a Model.

    x = Variable(float, 0, 10)   # continuous
    n = Variable(int, 0, 10)     # integer
    b = Variable(int, 0, 1)      # binary == integer bounded to [0, 1]

The first argument is the variable's type, given as a builtin Python type
(``float`` -> continuous, ``int`` -> integer); the strings
``"continuous"``/``"integer"`` are also still accepted. There is no
dedicated binary type on either side of the FFI boundary — a binary
variable is simply an integer variable bounded to [0, 1].

Creating a Variable immediately registers it in the Rust core's internal
storage (``model._core.add_variable``), which returns the column index
that will be used everywhere the variable's coefficient is stored in the
CSR matrices. Variable is itself a Function (the linear expression that is
just "1 times itself"), so all Function arithmetic and comparison
operators work on Variables for free.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Optional, Union

from .function import Function
from .types import normalize_vtype

if TYPE_CHECKING:
    from .model import Model


class Variable(Function):
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
        """The value assigned to this variable by the most recent
        ``Model.solve()`` call. Raises if the model hasn't been solved."""
        return self.model._variable_value(self.index)

    def __repr__(self) -> str:
        return f"Variable(name={self.name!r}, vtype={self.vtype!r}, lb={self.lb:g}, ub={self.ub:g})"
