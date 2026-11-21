"""rootgap の広い計測の設定 (analysis/rootgap_20261121)。def: 今の既定。glc: 集約行ごとに候補を全部残す旧来の生成 (ENOMOTO_MIP_GEN_OLD=1) +
集約行に持ち上げつきのナップサックの被覆も当てる (ENOMOTO_MIP_LIFTED_COVER=1)。README §4〜6。どれも 4 スレッド (run.py の TH)。"""
CFGS = {"def": {}, "glc": {"ENOMOTO_MIP_GEN_OLD": "1", "ENOMOTO_MIP_LIFTED_COVER": "1"}}
