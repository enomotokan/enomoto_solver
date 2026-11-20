"""SCIP-VB の計測の設定 (analysis/vb01_20261120)。def: 今の既定。vb01b: 試作 ENOMOTO_MIP_VB01=b (0-1 列どうしの含意を使う vbounds を
根の LP の直前に回す)。vb01r: ENOMOTO_MIP_VB01=r (根の SCIP 由来のヒューリスティクスの後に回す)。どれも 4 スレッド。"""
CFGS = {"def": {}, "vb01b": {"ENOMOTO_MIP_VB01": "b"}, "vb01r": {"ENOMOTO_MIP_VB01": "r"}}
