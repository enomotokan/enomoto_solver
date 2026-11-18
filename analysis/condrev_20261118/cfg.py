"""condrev の設定 (BRIEF.md): def = 既定 (環境変数なし)。
sbt = 強分岐の締め付けの束 (無条件)、sbtc = 同じ束を一般整数の列が 10 本以上の問題だけ (ENOMOTO_MIP_SBT_GENINT_MIN=10)。
sf0 = SUBFIX を止める (無条件)、sfcap = SUBFIX は残し、外側の根の SUBFIX のサブ MIP を時間の上限の 5% で打ち切る (ENOMOTO_T_MIP_SUBFIX_CAP=0.05)。
rb = 根のカットを相対の判定で止める (r) + 既定の b、割合 0.02。"""
SBT = {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1", "ENOMOTO_MIP_SB_POLY_RC": "1"}
CFGS = {
    "def": {},
    "sbt": dict(SBT),
    "sbtc": dict(SBT, ENOMOTO_MIP_SBT_GENINT_MIN="10"),
    "sf0": {"ENOMOTO_MIP_SUBFIX": "0"},
    "sfcap": {"ENOMOTO_T_MIP_SUBFIX_CAP": "0.05"},
    "rb": {"ENOMOTO_MIP_ROOTSTOP": "rb", "ENOMOTO_T_MIP_ROOTSTOP_RATE": "0.02"},
}
