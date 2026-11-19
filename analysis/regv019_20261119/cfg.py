"""0.1.9 からの悪化の切り分け (2026-11-19) の設定。
def = 今の既定、v019 = PyPI の 0.1.9 (analysis/final_20261118/setup_v019.sh の .v019env の python で動かす)。
off_<名前> = 0.1.9 の後に既定にした機能を 1 つだけ切る (戻し方は docs/improvement_history.md の各節)。all_off = 全部切る。
バグの直し (LexRed の締め・NotSolved の直し・整数に見える LP 解の解き直し) と片付けは切れない。"""
OFF = {
    "rtc": {"ENOMOTO_T_PROP_RELTOL_CONT": "0"},
    "pc": {"ENOMOTO_AGG_PIVOT_COL": "0"},
    "sf": {"ENOMOTO_MIP_SEP_FAST": "0"},
    "vub": {"ENOMOTO_MIP_VUBAGG": "0"},
    "orb": {"ENOMOTO_MIP_ORBPP": "0"},
    "can": {"ENOMOTO_IPM_G5CANCEL": "0"},
    "clq": {"ENOMOTO_MIP_CLQPRB": "0"},
    "seq": {"ENOMOTO_MIP_G5SEQ": "0"},
    "rfs": {"ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS": "inf"},
    "dom": {"ENOMOTO_MIP_DOMCOL": "0", "ENOMOTO_MIP_DOMCOL_PRED": "0", "ENOMOTO_MIP_DOMCOL_REV": "0", "ENOMOTO_MIP_DOMCOL_NOPROBE2": "0",
            "ENOMOTO_T_MIP_DOMCOL_MIN_FRAC": "0"},
    "s3g": {"ENOMOTO_MIP_S3GAP": "0"},
    "sw": {"ENOMOTO_T_MIP_SUBWAIT": "0"},
    "spl": {"ENOMOTO_MIP_SPARSIFY_LATE": "0"},
    "nw": {"ENOMOTO_MIP_RACE_ITERCAP_NOWAIT": "0"},
}
CFGS = {"def": {}, "v019": {}} | {f"off_{k}": v for k, v in OFF.items()}
CFGS["all_off"] = {k: v for d in OFF.values() for k, v in d.items()}
# 案 (試作): VUBAGG はまとめる列が全部の列の 5% 以上のときだけ (physiciansched6-2 2.0% は使わない、neos-787933 99% は使う)
CFGS["vmf"] = {"ENOMOTO_T_MIP_VUBAGG_MIN_FRAC": "0.05"}
# neos5 の二分探索の結果 (8f4a55ebd で SYMDYN=f を既定から外した) の確かめ: 今の版に SYMDYN=f を戻す
CFGS["symf"] = {"ENOMOTO_MIP_SYMDYN": "f"}
