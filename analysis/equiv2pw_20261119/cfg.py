"""EQUIV2_PWORK の確かめ (analysis/clqprbg_20261119): def = 今の既定、epw = 試作 ENOMOTO_MIP_EQUIV2_PWORK=1 (同値な組の巡の手間を本体の
プロービングの pwork と同じ重みで数え、0.5 秒相当で止める)。"""
CFGS = {
    "def": {},
    "epw": {"ENOMOTO_MIP_EQUIV2_PWORK": "1"},
}
