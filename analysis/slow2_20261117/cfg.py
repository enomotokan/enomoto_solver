"""SLOW2 の設定: def = 0.1.9 の既定 (環境変数なし)、wst = 空回りを除く試作、sw = 根の並列のサブ MIP の待ちの上限 (analysis/slow2_20261117/README.md)。"""
CFGS = {
    "def": {},
    "wst": {"ENOMOTO_MIP_PRR_PRECHECK": "1", "ENOMOTO_T_MIP_HNFPRE_REPRE_MIN": "10"},
    "sw": {"ENOMOTO_T_MIP_SUBWAIT": "0.5"},
}
