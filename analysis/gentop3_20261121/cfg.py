"""gentop3 の設定 (analysis/gentop3_20261121)。def: 今の既定。lc: 集約行に持ち上げつきの被覆 (ENOMOTO_MIP_LIFTED_COVER=1)。
t3: 集約行ごとに最良の質の 0.5 倍以上から良い順に最大 3 本残す (ENOMOTO_MIP_GEN_TOP3=1)。t3lc: t3 + LIFTED_COVER。
diag / diaglc: 集約行ごとの候補の数と 0.5 倍以上の数を出すだけ (ENOMOTO_MIP_GEN_TOP3_LOG=1、結果は既定・lc と同じ)。"""
CFGS = {
    "def": {},
    "lc": {"ENOMOTO_MIP_LIFTED_COVER": "1"},
    "t3": {"ENOMOTO_MIP_GEN_TOP3": "1"},
    "t3lc": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
    "diag": {"ENOMOTO_MIP_GEN_TOP3_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1"},
    "diaglc": {"ENOMOTO_MIP_GEN_TOP3_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
}
