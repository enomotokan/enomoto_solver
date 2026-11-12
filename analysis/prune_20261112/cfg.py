"""PRUNE (2026-11-12) の設定: 既定 (def) と、既定の部品を 1 つずつ止めた形 (leave-one-out)。
部品の選び方と出どころは analysis/prune_20261112.md の表。止め方は各部品の既定化の記録のとおり。"""
LOO = {
    "symdyn0": {"ENOMOTO_MIP_SYMDYN": "0"},                 # 動的な辞書式の縮小 (11-11、効き目ははっきりしない)
    "subfix0": {"ENOMOTO_MIP_SUBFIX": "0"},                 # 固定のサブ MIP の型 rtcqf (11-09 cand1108zd)
    "probrep0": {"ENOMOTO_MIP_PROBE_REPRESOLVE": "0"},      # プロービングの後の前処理のかけ直し (11-09 cand1108zd)
    "mpsep0": {"ENOMOTO_MIP_MPSEP": "0"},                   # 分離の型 oqmgs (11-09 cand1108zd)
    "hnf0": {"ENOMOTO_MIP_HNFPRE": "0"},                    # HNF の前処理 bounds (11-09 cand1108zd)
    "tabnnz0": {"ENOMOTO_MIP_TAB_NNZ_CBC": "0"},            # tableau のカットの非零の予算 (10-30)
    "sepamcf0": {"ENOMOTO_MIP_SEPA_MCF": "0"},              # MCF の cut-set カット (10-29)
    "orbx0": {"ENOMOTO_MIP_NO_ORBITOPE_TWO_COL": "1", "ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE": "1"},  # 2 列・双子のオービトープ (10-29)
    "cutagg0": {"ENOMOTO_MIP_CUT_AGG": "0"},                # 面の頂点の分離を止める f (10-24 fix9pe)
    "lpfin0": {"ENOMOTO_MIP_LP_PROBE_FINISH": "0"},         # LP プロービングの仕上げ (10-24 fix9pe)
    "grrst0": {"ENOMOTO_MIP_GR_RESTART": "0"},              # 一般整数の問題の再スタートの参照解・裏のカットの後の RINS (10-23)
    "fjp3off": {"ENOMOTO_MIP_NO_FJ_P3": "1"},               # FJ にだけ前処理をかけ直した問題を渡す (10-21)
}
CFGS = {"def": {}, **LOO}
