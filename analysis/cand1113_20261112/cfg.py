"""束 cand1113 (2026-11-12) の設定。既定 (def) に足す環境変数。WRONGANS の seed14 の直しは既定に入っているので両側に入る。
`highs` は同じ機械の HiGHS 1.15.1 の列 (Mittelmann の種 1 だけ)。"""
CAND1113 = {
    # 今日までに合格した項目
    "ENOMOTO_MIP_ERWAIT": "sbkc",                 # ewk
    "ENOMOTO_MIP_TREEEFF": "rpsh",                # tenc
    "ENOMOTO_MIP_NODECNT": "hr",                  # tenc
    "ENOMOTO_MIP_IPMRC": "utw",                   # utw
    "ENOMOTO_MIP_RSTCRIT": "n",                   # n05
    "ENOMOTO_MIP_FOLD": "rinq",                   # net
    "ENOMOTO_MIP_LMBOOST": "1",
    "ENOMOTO_MIP_PREPAR": "ple",                  # prepar
    "ENOMOTO_MIP_NODE_CUT_MAXLEN": "1",           # ncm
    "ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250",      # ns250s
    "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL": "1",   # ns250s
    "ENOMOTO_MIP_SUBFIX": "0",                    # prune1112
    "ENOMOTO_MIP_HNFPRE": "0",                    # prune1112
    "ENOMOTO_MIP_NO_ORBITOPE_TWO_COL": "1",       # prune1112
    "ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE": "1",      # prune1112
    "ENOMOTO_MIP_LP_PROBE_FINISH": "0",           # prune1112
    "ENOMOTO_MIP_DUALEQ": "1",
    # cand1112 の 7 項目
    "ENOMOTO_MIP_ROOTSTOP": "b",
    "ENOMOTO_MIP_RCSTRONG": "irk",
    "ENOMOTO_MIP_LMIP_SWAP": "1",
    "ENOMOTO_MIP_SYMPRE": "probe",
    "ENOMOTO_MIP_PLSB": "gxu",
    "ENOMOTO_MIP_GAP1": "u",
    "ENOMOTO_MIP_GAP4": "rocn",
    # 今日合格した項目
    "ENOMOTO_MIP_OBJINT": "hfc",
    "ENOMOTO_MIP_CLQPRB": "1d",
}
CFGS = {"def": {}, "cand1113": CAND1113, "highs": {}}
