"""比べる設定 (環境変数)。base は既定。base2 は既定の 2 本目 (揺れの目安)。"""
CFGS = {
    "base": {},
    "base2": {},
    # 木の探索
    "sb_tighten": {"ENOMOTO_MIP_SB_TIGHTEN": "1"},
    "sb_of": {"ENOMOTO_MIP_SB_OF": "1"},
    "sb_fixres": {"ENOMOTO_MIP_SB_FIX_RESOLVE": "1"},
    "zi_adapt": {"ENOMOTO_MIP_ZIROUND_ADAPT": "1"},
    "pc_cutoff": {"ENOMOTO_MIP_PC_COUNT_CUTOFF": "1"},
    "dive_rcfix": {"ENOMOTO_MIP_DIVE_RCFIX": "1"},
    "cost_prio": {"ENOMOTO_MIP_COST_PRIORITY": "1"},
    # カットのループ
    "keep_lu": {"ENOMOTO_MIP_CUT_KEEP_LU": "1"},
    "sep_adapt": {"ENOMOTO_MIP_SEP_ADAPTIVE": "1"},
    "origrow": {"ENOMOTO_MIP_ORIGROW_TIGHT": "1"},
    "sep_inv": {"ENOMOTO_MIP_SEP_PARALLEL": "1", "ENOMOTO_MIP_SEP_NO_ROWCLONE": "1"},
    "zh_pre": {"ENOMOTO_MIP_ZH_PRE": "1"},
    "tab_nnz": {"ENOMOTO_MIP_TAB_NNZ_CBC": "1"},
    "pool_sel": {"ENOMOTO_MIP_POOL_SELECTED_ONLY": "1"},
    "node_maxlen": {"ENOMOTO_MIP_NODE_CUT_MAXLEN": "1"},
    "fail_restore": {"ENOMOTO_MIP_CUT_FAIL_RESTORE": "1"},
    # 前処理・LP
    "probe_str": {"ENOMOTO_MIP_PROBE_STRENGTHEN": "1"},
    "ineq_all": {"ENOMOTO_MIP_INEQ_SINGLETON_ALL": "1"},
    "vubs": {"ENOMOTO_MIP_TIGHTEN_VUBS": "1"},
    "node_perturb": {"ENOMOTO_NODE_PERTURB": "1"},
}
