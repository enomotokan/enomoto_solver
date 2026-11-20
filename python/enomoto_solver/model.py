"""Model: 問題入力インターフェースの入口。

    M = Model()
    x = Variable(float, 0, 10)   # 現在の Model (M) に属する
    f = x + 2 * y
    M.set_objective(f)
    M.add_constraint(0 <= f)
    M.solve()

Model 自体は行列を持たず、変数・目的関数・制約の宣言を Rust コア (``self._core``) に
転送するだけ。行列の保持・前処理・最適化アルゴリズムは Rust コア側にある。
"""

from __future__ import annotations

import gzip
import os
import time
import warnings
from dataclasses import dataclass
from typing import ClassVar, List, Optional

from . import _core
from .constraint import Constraint
from .function import Function


def _version() -> str:
    """インストールされているパッケージの版 (分からなければ空文字列)。"""
    try:
        from importlib.metadata import version

        return version("enomoto-solver")
    except Exception:
        return ""


# ``Model.solve(log=True)`` で最初に出す案内
_BANNER = (
    "ENOMOTO-Solver {version} (MIT License)\n"
    "Author: Kan Enomoto (https://enomoto-math-labo.com/en)\n"
    "For optimization consulting requests and bug reports, contact enomotokan@gmail.com\n"
)


def _fmt(v: Optional[float]) -> str:
    """ログに出す数値 (None は "-")。"""
    return "-" if v is None else f"{v:.10g}"


@dataclass(frozen=True)
class Solution:
    """``Model.solve()`` の結果。

    属性:
        status: ``"optimal"`` / ``"infeasible"`` / ``"unbounded"`` /
            ``"infeasible_or_unbounded"`` / ``"not_solved"`` のいずれか。
            制限時間で打ち切ると ``"time_limit"`` (整数最適化では暫定解があれば値が読める)、整数最適化ではさらに
            ``"node_limit"``。
        objective: 解の目的値 (``"optimal"``、または打ち切り時に暫定解があるとき。それ以外は None)。
        node_limit_hit: 整数最適化でノード数上限により打ち切られたか (最適性は未証明)。
        best_bound: 整数最適化で証明済みの最良の限界 (最小化なら下界)。LP では None。
        mip_gap: 整数最適化の相対ギャップ。LP では None。
        nodes: 整数最適化で処理したノード数。LP では None。
    """

    status: str
    objective: Optional[float]
    node_limit_hit: bool
    best_bound: Optional[float] = None
    mip_gap: Optional[float] = None
    nodes: Optional[int] = None


class Model:
    """最適化モデル。``Model()`` を作るとそれが *現在の* Model になり、``model=`` を
    指定しない ``Variable(...)`` は、最後に作られた (または ``with model:`` で入った)
    Model に属する。明示したい場合は ``Variable(..., model=...)`` を使う。"""

    # 現在の Model のスタック (末尾が現在の Model)
    _stack: ClassVar[List["Model"]] = []

    def __init__(self):
        """空のモデルを作り、現在の Model にする。"""
        self._core = _core.PyModel()
        self._variables: List["Variable"] = []  # noqa: F821 - 循環 import を避けるため Variable は遅延 import
        self._objective: Optional[Function] = None
        self._sense = "minimize"
        self._constraints: List[Constraint] = []
        self._solution: Optional[Solution] = None
        self._name = ""  # 問題の名前 (MPS の NAME。readModel で設定)
        self._names: Optional[dict] = None  # get_variable 用の 名前 -> Variable (遅延して作る)
        Model._stack.append(self)

    @classmethod
    def current(cls) -> "Model":
        """現在の Model を返す。1 つもなければ RuntimeError。"""
        if not cls._stack:
            raise RuntimeError(
                "no active Model — create one with `M = Model()` before defining Variables, "
                "or pass model=<Model> explicitly to Variable(...)"
            )
        return cls._stack[-1]

    def __enter__(self) -> "Model":
        """``with model:`` の間、このモデルを現在の Model にする。"""
        Model._stack.append(self)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        """``with`` ブロックを抜けたら現在の Model を元に戻す。"""
        Model._stack.pop()

    def _register_variable(self, var) -> None:
        """Variable の生成時に呼ばれ、変数を Python 側の一覧に加える。"""
        self._variables.append(var)

    def _variable_value(self, index: int) -> float:
        """変数番号 ``index`` の解の値。解がなければ RuntimeError。"""
        if self._solution is None or self._solution_x is None:
            raise RuntimeError("Model has no solution yet — call Model.solve() first (no solution is available unless it found one)")
        return self._solution_x[index]

    # -- 問題の定義 --------------------------------------------------------
    def set_objective(self, f: Function, sense: str = "minimize") -> None:
        """目的関数 ``f`` と向き (``"minimize"`` / ``"maximize"``) を設定する
        (2 回目以降は上書き)。"""
        if not isinstance(f, Function):
            raise TypeError(f"Model.set_objective expects a Function, got {type(f).__name__}")
        if sense not in ("minimize", "maximize"):
            raise ValueError("sense must be 'minimize' or 'maximize'")
        self._objective = f
        self._sense = sense
        self._core.set_objective(f.nonzero_terms(), f.constant, sense)

    def add_constraint(self, g: Constraint) -> None:
        """制約 ``g`` を追加する。"""
        if not isinstance(g, Constraint):
            raise TypeError(f"Model.add_constraint expects a Constraint, got {type(g).__name__}")
        self._constraints.append(g)
        self._core.add_constraint(g.nonzero_terms(), g.sense, g.rhs)

    def readModel(self, path: "str | os.PathLike[str]") -> None:
        """MPS ファイル ``path`` (``.mps`` / 末尾 ``.gz`` なら gzip 圧縮) を読み、この空の Model に
        変数・目的関数・制約を入れる。読み取りは Rust コア (``src/mps.rs``) が行う。

            M = Model()
            M.readModel("neos-911970.mps.gz")
            sol = M.solve()
            x = M.get_variable("C0001")
            print(sol.objective, x.value)

        変数は MPS の列の順に ``M.variables`` に並び、名前は MPS の列の名前。
        ``MARKER`` で囲まれた整数列は、``BOUNDS`` に何も書かれなければ境界 ``[0, 1]`` (HiGHS と同じ約束)。
        範囲のある行 (``RANGES``) は ``<=`` と ``>=`` の 2 本の制約になる。係数はファイルどおりに読む
        (HiGHS のように絶対値 1e-9 以下の係数を捨てることはしない)。定義されていない行・列への
        参照は警告 (``warnings.warn``) を出して飛ばす。二次の項・半連続変数・SOS は ValueError。
        読み込んだ後に ``Variable`` や ``add_constraint`` で変数・制約を足してもよい。
        変数・目的関数・制約のどれかが既にある Model では ValueError。"""
        from .variable import Variable

        if self._variables or self._constraints or self._objective is not None or self._core.n_variables():
            raise ValueError("Model.readModel needs an empty Model (create a new one with Model())")
        p = os.fspath(path)
        opener = gzip.open if p.endswith(".gz") else open
        with opener(p, "rt", encoding="latin-1") as fh:
            text = fh.read()
        name, col_names, notes = self._core.read_mps(text)
        if notes:
            # 定義されていない行・列への参照は飛ばして読む (HiGHS と同じ)。最初の数件だけ見せる
            shown = "; ".join(notes[:3]) + (f"; ... ({len(notes)} in total)" if len(notes) > 3 else "")
            warnings.warn(f"readModel({p!r}): {shown}", stacklevel=2)
        lb, ub, is_int = self._core.variable_bounds()
        # Variable.__init__ は Rust コアに変数を足すので通らず、登録済みの列を指す Variable を作る
        for j, cname in enumerate(col_names):
            v = Variable.__new__(Variable)
            Function.__init__(v, self, {j: 1.0}, 0.0)
            v.vtype = "integer" if is_int[j] else "continuous"
            v.lb = lb[j]
            v.ub = ub[j]
            v.index = j
            v.name = cname
            self._variables.append(v)
        coeffs, constant, sense = self._core.objective_terms()
        self._objective = Function(self, dict(coeffs), constant)
        self._sense = sense
        self._name = name
        self._names = None

    # MPS を読む口の別名 (このパッケージの他のメソッドに合わせた snake_case)
    read_model = readModel

    @property
    def variables(self) -> List["Variable"]:  # noqa: F821
        """登録順 (MPS を読んだときは列の順) の変数の一覧 (複製)。"""
        return list(self._variables)

    def get_variable(self, name: str) -> "Variable":  # noqa: F821
        """名前が ``name`` の変数を返す。見つからなければ KeyError。"""
        names = self._names
        if names is None or len(names) != len(self._variables):
            names = {v.name: v for v in self._variables}
            self._names = names
        return names[name]

    # -- 求解 ------------------------------------------------------------
    def solve(
        self,
        lp_solver: Optional[str] = None,
        distinguish_infeasible_unbounded: bool = False,
        time_limit: Optional[float] = None,
        mip_rel_gap: Optional[float] = None,
        node_limit: Optional[int] = None,
        threads: Optional[int] = None,
        log: bool = True,
        *,
        root_solver: Optional[str] = None,
    ) -> Solution:
        """Rust コアで前処理と最適化を実行し、結果を Solution で返す。

        最適解が得られたかどうかにかかわらず例外は送出しない。結果は ``Solution.status``
        で判断する。``"optimal"`` のときだけ ``objective`` に値が入り、各 Variable の
        ``.value`` が読める (それ以外で ``.value`` を読むと RuntimeError)。

        ``lp_solver``: LP (整数変数を含まないモデル) の解法。整数最適化の緩和問題は常に傾き・切片双対二段解法を
        warm start で使って解くので、整数変数を含むモデルに ``"auto"`` (と ``None``) 以外を渡しても効かず、
        ``UserWarning`` を出す (求解はそのまま続ける)。

        - ``"auto"`` (既定、``None`` も同じ): ``threads`` が 2 以上なら、傾き・切片双対二段解法 (1 スレッド) と
          内点法 + クロスオーバー (残りのスレッド) を同時に解き、先に結論を出した側を採る (もう一方は打ち切る)。
          1 なら傾き・切片双対二段解法だけで解く。実行不能・非有界の判定は常に二段解法が出す
          (内点法 + クロスオーバーが採られるのは、単体法で最適性を確かめた基底解が得られたときだけ)。
        - ``"simplex"``: 傾き・切片双対二段解法だけ。
        - ``"ipm_crossover"``: 内点法 + クロスオーバー (Liu & Lu 2024) だけ。内点法が収束しなければ
          二段解法で解き直す。
        - ``"interior"``: 前処理も独立の IP-PMM 内点法 (基底解ではない。突き合わせ検証用)。

        ``distinguish_infeasible_unbounded``: 単体法の結果状態は既定では ``"optimal"``、
        ``"infeasible"``、``"infeasible_or_unbounded"`` のいずれか。拡張双対単体法の
        段階 A が ``z^1 < 0`` で終われば有限の最適値がないことが証明されてそこで止まり、
        ``z^1 = 0`` なら非有界ではなく、段階 B で最適解を求めるか実行不能を証明する。
        True を渡すと前者も ``"infeasible"`` と ``"unbounded"`` に分ける。整数最適化では、根の緩和問題で
        ``z^1 < 0`` のとき、切片問題 (段階 B の問題) の変数を整数に制限した問題を解き、実行可能解があれば
        ``"unbounded"``、実行不能なら ``"infeasible"`` を返す (False なら ``"infeasible_or_unbounded"``)。
        ``"not_solved"`` はソルバーが判定に至らずに諦めたことを表す。

        ``time_limit`` (秒): 制限時間。LP では、期限までに結論が出なければ status ``"time_limit"`` で返す
        (前処理の途中では止まらないので、前処理が長い問題では少し超えることがある)。

        整数最適化の打ち切り条件: ``time_limit`` (秒)、``mip_rel_gap`` (相対ギャップ、既定 0)、
        ``node_limit`` (ノード数)。上限で止まると status は ``"time_limit"`` / ``"node_limit"`` になり、
        暫定解があれば ``objective`` と各 Variable の ``.value`` が読める。

        ``threads``: 使うスレッドの数 (既定は ``enomoto_solver.default_threads()``: 使える CPU の数 + 1 の半分。HiGHS と同じ)。``enomoto_solver.max_threads()`` (このプロセスが使える CPU の数) を超える値はそこで頭打ちにする。
        求解全体をこの数のスレッドで動かす。``"auto"`` で 2 以上なら、LP を上のように二段解法と内点法 +
        クロスオーバーで同時に解く。整数最適化では warm start できない緩和問題 (根など) を同じように同時に解き、根の処理の後は未処理のノードを
        スレッドの間で分け合って探索する。

        ``log`` (既定 True): 最初に作者・ライセンス・連絡先の案内と問題の大きさを、求解中は要点 (前処理の結果、
        同時実行の勝敗、整数最適化では根の LP・切除平面・探索の経過) を、最後に結果を標準出力に出す。
        False なら何も出さない。

        ``root_solver``: ``lp_solver`` の以前の名前 (互換のため残す。使うと ``DeprecationWarning`` を出す)。
        ``lp_solver`` と同時に指定すると ``TypeError``。
        """
        if root_solver is not None:
            if lp_solver is not None:
                raise TypeError("specify only one of lp_solver and root_solver (root_solver is the deprecated name of lp_solver)")
            warnings.warn(
                "root_solver is deprecated; use lp_solver instead",
                DeprecationWarning,
                stacklevel=2,
            )
            lp_solver = root_solver
        if lp_solver not in (None, "auto") and any(v.vtype == "integer" for v in self._variables):
            warnings.warn(
                f"lp_solver={lp_solver!r} has no effect on a model with integer variables: "
                "the relaxations of an integer program are always solved by the slope-intercept dual two-phase method",
                UserWarning,
                stacklevel=2,
            )
        if log and self._objective is not None:
            self._log_header(threads)
        t0 = time.perf_counter()
        result = self._core.solve(
            lp_solver=lp_solver,
            distinguish_infeasible_unbounded=distinguish_infeasible_unbounded,
            time_limit=time_limit,
            mip_rel_gap=mip_rel_gap,
            node_limit=node_limit,
            threads=threads,
            log=log,
        )
        elapsed = time.perf_counter() - t0
        status = result["status"]
        self._solution_x = result["x"]
        self._solution = Solution(
            status=status,
            objective=result["objective"],
            node_limit_hit=result["node_limit_hit"],
            best_bound=result["best_bound"],
            mip_gap=result["mip_gap"],
            nodes=result["nodes"],
        )
        if log:
            self._log_result(self._solution, elapsed)
        return self._solution

    def _log_header(self, threads: Optional[int]) -> None:
        """``solve(log=True)`` の最初の案内と問題の大きさを出す。"""
        n_int = sum(1 for v in self._variables if v.vtype == "integer")
        nnz = self._core.n_nonzeros()
        used = min(max(threads or _core.default_threads(), 1), _core.max_threads())
        print(_BANNER.format(version=_version()).replace("  ", " "), flush=True)
        print(
            f"Problem: {self._core.n_variables()} variables ({n_int} integer), {self._core.n_constraints()} constraints, "
            f"{nnz} nonzeros, {self._sense}; threads {used}",
            flush=True,
        )

    def _log_result(self, sol: Solution, elapsed: float) -> None:
        """``solve(log=True)`` の最後の結果の行を出す。"""
        line = f"Result: {sol.status}, objective {_fmt(sol.objective)}"
        if sol.nodes is not None:
            gap = "-" if sol.mip_gap is None or sol.mip_gap == float("inf") else f"{100 * sol.mip_gap:.2f}%"
            line += f", best bound {_fmt(sol.best_bound)}, gap {gap}, nodes {sol.nodes}"
        print(f"{line} ({elapsed:.2f}s)", flush=True)

    def __repr__(self) -> str:
        """変数数・制約数・目的関数の設定有無を示す表示用文字列。"""
        return (
            f"Model(variables={self._core.n_variables()}, "
            f"constraints={self._core.n_constraints()}, "
            f"objective={'set' if self._objective is not None else 'unset'})"
        )
