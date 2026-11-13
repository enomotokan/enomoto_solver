"""束 cand1114 (2026-11-13)。cand1113 (analysis/cand1113_20261112/cfg.py) を組み直した既定化の候補。既定 (def) に足す環境変数。
cand1113 からの変更 (理由は README.md の「組み直し」の節):
  RCSTRONG irk → k (RCSTRONG2)、外す: HNFPRE=0 (enlight_hard)・SUBFIX=0 (neos-960392)・IPMRC=utw (neos-1122047)・DUALEQ (binkar10_1)・CLQPRB (roll3000・cbs-cta)。
誤答の直し 40e80904 (unitcal_7) と UNB_FEAS (dc1f6842) は既定に入っているので両側に入る。`highs` は同じ機械の HiGHS 1.15.1 の列 (M の種 1 だけ)。"""
CAND1114 = {
    "ENOMOTO_MIP_ERWAIT": "sbkc",                 # ewk
    "ENOMOTO_MIP_TREEEFF": "rpsh",                # tenc
    "ENOMOTO_MIP_NODECNT": "hr",                  # tenc
    "ENOMOTO_MIP_RSTCRIT": "n",                   # n05
    "ENOMOTO_MIP_FOLD": "rinq",                   # net
    "ENOMOTO_MIP_LMBOOST": "1",
    "ENOMOTO_MIP_PREPAR": "ple",                  # prepar
    "ENOMOTO_MIP_NODE_CUT_MAXLEN": "1",           # ncm
    "ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250",      # ns250s
    "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL": "1",   # ns250s
    "ENOMOTO_MIP_NO_ORBITOPE_TWO_COL": "1",       # prune1112 (SUBFIX=0・HNFPRE=0 は外した)
    "ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE": "1",      # prune1112
    "ENOMOTO_MIP_LP_PROBE_FINISH": "0",           # prune1112
    "ENOMOTO_MIP_ROOTSTOP": "b",
    "ENOMOTO_MIP_RCSTRONG": "k",                  # RCSTRONG2 (cand1113 は irk)
    "ENOMOTO_MIP_LMIP_SWAP": "1",
    "ENOMOTO_MIP_SYMPRE": "probe",
    "ENOMOTO_MIP_PLSB": "gxu",
    "ENOMOTO_MIP_GAP1": "u",
    "ENOMOTO_MIP_GAP4": "rocn",
    "ENOMOTO_MIP_OBJINT": "hfc",
}
CFGS = {"def": {}, "cand1114": CAND1114, "highs": {}}
