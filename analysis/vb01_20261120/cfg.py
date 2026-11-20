"""SCIP-VB の計測の設定 (analysis/vb01_20261120)。def: 今の既定。vb: 試作 ENOMOTO_MIP_VB01=b (0-1 列どうしの含意を使う vbounds を
根の LP の直前に回す。SCIP-GATE (analysis/scipgate_20261120) の vb と同じ設定・同じ名前)。(`r` は外した。)どれも 4 スレッド。"""
CFGS = {"def": {}, "vb": {"ENOMOTO_MIP_VB01": "b"}}
