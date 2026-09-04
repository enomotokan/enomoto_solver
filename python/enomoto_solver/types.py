"""Small shared helpers for the Python-side input interface."""

from __future__ import annotations

_ALIASES = {
    "continuous": "continuous",
    "cont": "continuous",
    "c": "continuous",
    "real": "continuous",
    "integer": "integer",
    "int": "integer",
    "i": "integer",
}

# Variable("continuous"/"integer", lb, ub) is still accepted, but the primary
# spelling is the builtin type itself: Variable(float, ...) / Variable(int, ...).
# There is no dedicated binary vtype (on either side) — a binary variable is
# just an integer variable bounded to [0, 1]: Variable(int, 0, 1).
_TYPE_ALIASES = {
    float: "continuous",
    int: "integer",
}


def normalize_vtype(vtype) -> str:
    """Normalizes a user-supplied variable type — either a builtin type
    (``int`` / ``float``) or a string spelling — to one of
    'continuous' | 'integer', which is what the Rust core
    (``VarType::parse``) understands."""
    if isinstance(vtype, type):
        try:
            return _TYPE_ALIASES[vtype]
        except KeyError:
            raise ValueError(
                f"unknown variable type {vtype!r}; expected one of "
                "int, float (or the strings 'continuous'/'integer')"
            ) from None
    if isinstance(vtype, str):
        key = vtype.strip().lower()
        try:
            return _ALIASES[key]
        except KeyError:
            raise ValueError(
                f"unknown variable type {vtype!r}; expected one of "
                "'continuous', 'integer' (or short aliases 'c'/'i')"
            ) from None
    raise TypeError(
        f"variable type must be int, float, or a string, got {type(vtype).__name__}"
    )
