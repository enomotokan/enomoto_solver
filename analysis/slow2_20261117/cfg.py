"""SLOW2 の設定: def = 0.1.9 の既定 (環境変数なし)、wst = 空回りを除く試作、sw = 根の並列のサブ MIP の待ちの上限、vbis = 根のカットの変数上下限と選び方 (analysis/slow2_20261117/README.md)。"""
CFGS = {
    "def": {},
    "wst": {"ENOMOTO_MIP_PRR_PRECHECK": "1", "ENOMOTO_T_MIP_HNFPRE_REPRE_MIN": "10"},
    "sw": {"ENOMOTO_T_MIP_SUBWAIT": "0.5"},
    # vb = VB_IMPL だけ (README 10 節。vbis の P の害は選び方の変更の側から来ていた)
    "vb": {"ENOMOTO_MIP_VB_IMPL": "1"},
    # vbis = プロービングの含意を連続列の変数上下限に足す (VB_IMPL) + implied bound も選択に通す + 選び方の緩和・ラウンド数・第 2 段階 (README 9 節)
    "vbis": {"ENOMOTO_MIP_VB_IMPL": "1", "ENOMOTO_MIP_IMPLBD_SELECT": "1", "ENOMOTO_T_CUTSEL_MIN_REL": "0.1", "ENOMOTO_T_MIP_ROOT_MAX_CUTS": "200",
             "ENOMOTO_T_MIP_CUT_ROUNDS": "60", "ENOMOTO_MIP_CUT_PHASE2": "1"},
}
