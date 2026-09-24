"""モデルの構築・求解で送出される例外。"""


class SolverError(Exception):
    """モデルの構築・求解中のエラーの基底クラス。"""


class InfeasibleError(SolverError):
    """Model.solve() で、問題に実行可能解がないときに送出される。"""


class UnboundedError(SolverError):
    """Model.solve() で、目的関数が非有界のときに送出される。"""


class InfeasibleOrUnboundedError(SolverError):
    """Model.solve() で、有限の最適値を持たないこと (実行不能か非有界) は証明したが、
    どちらかは区別していないときに送出される (既定の動作。
    ``distinguish_infeasible_unbounded=True`` を渡すと InfeasibleError か
    UnboundedError のどちらかになる)。どちらでもありうるので、意図的に
    どちらのサブクラスにもしていない。"""


class NotSolvedError(SolverError):
    """Model.solve() で、ソルバーが判定に至らずに諦めたときに送出される
    (回復できない数値的に特異な基底や反復上限など)。モデル自体については何も主張しない。"""
