"""GATE の設定: def = 既定 (環境変数なし)、sw = 根の並列のサブ MIP の待ちの上限 k=0.5 (analysis/slow2_20261117/README.md の 8 節)。"""
CFGS = {
    "def": {},
    "sw": {"ENOMOTO_T_MIP_SUBWAIT": "0.5"},
}
