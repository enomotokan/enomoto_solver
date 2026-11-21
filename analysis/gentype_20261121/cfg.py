"""gentype の設定 (analysis/gentype_20261121)。def: 今の既定。glc: 旧来の生成 (ENOMOTO_MIP_GEN_OLD=1) + 集約行に持ち上げつきの被覆
(ENOMOTO_MIP_LIFTED_COVER=1、rootgap_20261121 と同じ)。gt: 集約行ごとに種類ごとの最良を残す (ENOMOTO_MIP_GEN_TYPE=1)。gtlc: gt + LIFTED_COVER。"""
CFGS = {
    "def": {},
    "glc": {"ENOMOTO_MIP_GEN_OLD": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
    "gt": {"ENOMOTO_MIP_GEN_TYPE": "1"},
    "gtlc": {"ENOMOTO_MIP_GEN_TYPE": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"},
}
