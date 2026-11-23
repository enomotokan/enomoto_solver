"""ENOMOTO-Solver: 線形最適化・混合整数最適化ソルバーの Python 入力インターフェース。
行列の保持・前処理・最適化アルゴリズムは Rust コア (``enomoto_solver._core``) が担う。

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

def _enomoto_load_core():
    """wheel に x86-64-v3 向けの ``_core_v3`` が同梱されていて、CPU が v3 に対応していれば、それを
    ``enomoto_solver._core`` として読む (配る wheel の作り方は analysis/buildopt_20261122)。

    判定は基本の命令で作った ``_core`` の ``cpu_x86_64_v3()`` (Rust の ``is_x86_feature_detected!``) が行うので、
    Linux・Windows・macOS で同じに動く。``_core_v3`` が無い wheel (arm64、手元の ``maturin develop`` など) では何もしない。
    環境変数 ``ENOMOTO_CORE`` で強制できる: ``auto`` (既定)・``base`` (常に ``_core``)・``v3`` (CPU を見ずに ``_core_v3``)。
    ほかのモジュールが ``_core`` を import するより前に動かす。
    """
    import importlib.util
    import os
    import sys
    import warnings

    want = os.environ.get("ENOMOTO_CORE", "auto").strip().lower() or "auto"
    if want not in ("auto", "base", "v3"):
        warnings.warn(f"ENOMOTO_CORE={want!r} は auto・base・v3 のどれでもないので auto とみなす", RuntimeWarning, stacklevel=3)
        want = "auto"
    if want == "base":
        return
    here = os.path.dirname(os.path.abspath(__file__))
    cands = sorted(
        os.path.join(here, f) for f in os.listdir(here) if f.startswith("_core_v3.") and f.endswith((".so", ".pyd"))
    )
    if not cands:
        if want == "v3":
            warnings.warn("ENOMOTO_CORE=v3 だが _core_v3 が同梱されていないので _core を使う", RuntimeWarning, stacklevel=3)
        return
    name = __name__ + "._core"
    if want == "auto":
        from . import _core as base_core

        if not getattr(base_core, "cpu_x86_64_v3", lambda: False)():
            return
    spec = importlib.util.spec_from_file_location(name, cands[0])
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    sys.modules[name] = mod
    globals()["_core"] = mod


_enomoto_load_core()
del _enomoto_load_core

from .constraint import Constraint
from .function import Function
from .model import Model, Solution
from .variable import Variable
from ._core import __version__, default_threads, max_threads

# 公開 API
__all__ = [
    "default_threads",
    "max_threads",
    "Model",
    "Variable",
    "Function",
    "Constraint",
    "Solution",
]

# パッケージのバージョン (`__version__`) は Rust コアが Cargo.toml の版を持つので、それを上で読み込む。
# 版を上げるときは Cargo.toml だけを変える (pyproject.toml の version も同じ所から取る)。
