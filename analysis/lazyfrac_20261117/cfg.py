"""試作 LAZYFRAC (外せる行の非零の割合 f のシグモイドで、GENLAZY・CLQ_LAZY の外す量を決める) の設定。
- gs: GENLAZY の点数の判定 (τ 0) の候補の非零の割合 f で、余裕の大きい順に σ(40 (f − 0.3)) の割合だけ外す。
- cs: CLQ_LAZY の θ 0.1 (文字の和 < 0.1、`ENOMOTO_T_MIP_CLQ_LAZY_TOL` 0.9) の候補を、余裕のあるクリーク型の行全部 (θ = 1) の非零の割合 f で
  σ(150 (f − 0.12)) の割合だけ外す (`ENOMOTO_MIP_CLQ_LAZY_GATE_FALL=1`。θ 0.1 の候補で数えると neos-933966 と supportcase33 が分かれない)。
- g1: 対照。gs と同じ門 (f >= 0.3) を満たす問題で、余裕の最も大きい 1 行だけ外す (`ENOMOTO_MIP_LAZY_GATE_ONE=1`。mzzv11 の
  「1 行外すだけで速い」偶然の分を測る)。
- f0 は genlazy・clqtheta の種 1〜3 の結果を見て選んだ (楽観的)。計測はそれに使っていない種で行う。
lazy の設定は診断 (`ENOMOTO_MIP_DIAG_GENLAZY=1`: GENLAZY / CLQLAZY_F の行、門の行、終わりの数) も出す。"""
def gen(f0=0.3, k=40, one=False, tau=0.0):
    e = {"ENOMOTO_MIP_GEN_LAZY": "1", "ENOMOTO_MIP_GEN_LAZY_MODE": "score", "ENOMOTO_T_MIP_GEN_LAZY_TAU": str(tau), "ENOMOTO_MIP_DIAG_GENLAZY": "1",
         "ENOMOTO_T_MIP_GEN_LAZY_GATE_F0": str(f0), "ENOMOTO_T_MIP_GEN_LAZY_GATE_K": str(k)}
    if one:
        e["ENOMOTO_MIP_LAZY_GATE_ONE"] = "1"
    return e
def clq(theta=0.1, f0=0.12, k=150, one=False):
    e = {"ENOMOTO_MIP_CLQ_LAZY": "1", "ENOMOTO_T_MIP_CLQ_LAZY_TOL": str(1 - theta), "ENOMOTO_MIP_DIAG_GENLAZY": "1", "ENOMOTO_MIP_CLQ_LAZY_GATE_FALL": "1",
         "ENOMOTO_T_MIP_CLQ_LAZY_GATE_F0": str(f0), "ENOMOTO_T_MIP_CLQ_LAZY_GATE_K": str(k)}
    if one:
        e["ENOMOTO_MIP_LAZY_GATE_ONE"] = "1"
    return e
CFGS = {
    "def": {},
    "gs": gen(), "cs": clq(), "g1": gen(one=True),
    # 門なしの形 (genlazy の s00、clqtheta の t01 と同じ。手元の比べ用)
    "s00": {"ENOMOTO_MIP_GEN_LAZY": "1", "ENOMOTO_MIP_GEN_LAZY_MODE": "score", "ENOMOTO_T_MIP_GEN_LAZY_TAU": "0", "ENOMOTO_MIP_DIAG_GENLAZY": "1"},
    "c01": {"ENOMOTO_MIP_CLQ_LAZY": "1", "ENOMOTO_T_MIP_CLQ_LAZY_TOL": "0.9", "ENOMOTO_MIP_DIAG_GENLAZY": "1"},
}
