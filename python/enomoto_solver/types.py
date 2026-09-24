"""Python 側入力インターフェースの小さな共通補助関数。"""

from __future__ import annotations

# 変数型の文字列表記 (小文字化後) -> 正規名
_ALIASES = {
    "continuous": "continuous",
    "cont": "continuous",
    "c": "continuous",
    "real": "continuous",
    "integer": "integer",
    "int": "integer",
    "i": "integer",
}

# 組み込み型 -> 正規名。Variable(float, ...) / Variable(int, ...) が基本の書き方
# (文字列 "continuous"/"integer" も受け付ける)。二値専用の型はなく、
# 二値変数は境界 [0, 1] の整数変数 Variable(int, 0, 1) で表す。
_TYPE_ALIASES = {
    float: "continuous",
    int: "integer",
}


def normalize_vtype(vtype) -> str:
    """ユーザー指定の変数型 (組み込み型 ``int`` / ``float``、または文字列表記) を、
    Rust コア (``VarType::parse``) が理解する 'continuous' | 'integer' に正規化する。
    未知の型・表記なら ValueError、型でも文字列でもなければ TypeError。"""
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
