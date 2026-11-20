"""SCIP-VB の計測の設定 (analysis/vb01_20261120)。def: 今の既定。vb01: 試作 ENOMOTO_MIP_VB01=1 (0-1 列どうしの含意を使う vbounds)。どれも 4 スレッド。"""
CFGS = {"def": {}, "vb01": {"ENOMOTO_MIP_VB01": "1"}}
