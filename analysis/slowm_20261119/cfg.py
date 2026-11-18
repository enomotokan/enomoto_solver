"""P2-SCIP-M の計測の設定 (analysis/slowm_20261119)。def: 今の既定。fix: カットの締め付けの誤答の直し (ENOMOTO_MIP_TIGHTEN_INTBOUND=1)。
fixis: 直し + 試作 ENOMOTO_MIP_INTSING=1 (等式 1 本にだけ現れる整数列の消去)。どれも 4 スレッド。"""
CFGS = {"def": {}, "fix": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1"}, "fixis": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1", "ENOMOTO_MIP_INTSING": "1"}}
