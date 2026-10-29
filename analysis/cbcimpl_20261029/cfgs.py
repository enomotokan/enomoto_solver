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
    # 強分岐の境界の引き継ぎ (2 段目の計測)
    "sb_inh": {"ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1"},
    "sbt_a": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1"},
    "sbt_ab": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1"},
    # 3 段目: (b) を列ごとの折れ線に作り直したもの (コミット ac8cfc08 以降のビルド)。sbt_abp は 2 段目の sbt_ab と同じ環境変数だが中身が違う
    "sbt_abp": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1"},
    "sbt_abp_rc": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1", "ENOMOTO_MIP_SB_RC": "1"},
    "sbt_abp_rcp": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1", "ENOMOTO_MIP_SB_RC": "1", "ENOMOTO_MIP_SB_TIGHTEN_PROP": "1"},
    "sbt_abpr": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1", "ENOMOTO_MIP_SB_POLY_RC": "1"},
    # 5 段目 (確かめ): 1 段目で両方の組で良い側に出たものの組み合わせ
    "combo": {"ENOMOTO_MIP_TAB_NNZ_CBC": "1", "ENOMOTO_MIP_NODE_CUT_MAXLEN": "1", "ENOMOTO_MIP_DIVE_RCFIX": "1", "ENOMOTO_MIP_ZH_PRE": "1",
              "ENOMOTO_MIP_ORIGROW_TIGHT": "1", "ENOMOTO_MIP_COST_PRIORITY": "1"},
}
