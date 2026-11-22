"""gentop3 の設定 (analysis/gentop3_20261121)。def: 今の既定。lc: 集約行に持ち上げつきの被覆 (ENOMOTO_MIP_LIFTED_COVER=1)。
t3: 集約行ごとに最良の質の 0.5 倍以上から良い順に最大 3 本残す (ENOMOTO_MIP_GEN_TOP3=1)。t3lc: t3 + LIFTED_COVER。
diag / diaglc: 集約行ごとの候補の数と 0.5 倍以上の数を出すだけ (ENOMOTO_MIP_GEN_TOP3_LOG=1、結果は既定・lc と同じ)。
r5〜r9・r10: t3 の倍率の掃引 (GEN_TOP3=1 + GEN_TOP3 専用の倍率 ENOMOTO_T_MIP_GEN_TOP3_REL = 0.5〜0.9・1.0、6 節)。r5 は t3 と同じ動き。"""
CFGS = {
    "def": {},
    "lc": {"ENOMOTO_MIP_LIFTED_COVER": "1"},
    "t3": {"ENOMOTO_MIP_GEN_TOP3": "1"},
    "t3lc": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
    "diag": {"ENOMOTO_MIP_GEN_TOP3_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1"},
    "diaglc": {"ENOMOTO_MIP_GEN_TOP3_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
    "r5": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "0.5"},
    "r6": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "0.6"},
    "r7": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "0.7"},
    "r8": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "0.8"},
    "r9": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "0.9"},
    "r10": {"ENOMOTO_MIP_GEN_TOP3": "1", "ENOMOTO_T_MIP_GEN_TOP3_REL": "1.0"},
}
