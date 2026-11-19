"""P2-SCIP-M の計測の設定 (analysis/slowm_20261119)。def: 今の既定。fix: カットの締め付けの誤答の直し (ENOMOTO_MIP_TIGHTEN_INTBOUND=1)。
fixis: 直し + 試作 ENOMOTO_MIP_INTSING=1 (等式 1 本にだけ現れる整数列の消去)。st5: 根のカットの整数刻みの停滞の打ち切りを 3 → 5 巡
(ENOMOTO_T_MIP_CUT_INTOBJ_STALL=5、neos-3381206-awhea 1 スレッド 30.7 → 3.5 秒)。どれも 4 スレッド。"""
CFGS = {"def": {}, "fix": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1"}, "fixis": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1", "ENOMOTO_MIP_INTSING": "1"},
        "st5": {"ENOMOTO_T_MIP_CUT_INTOBJ_STALL": "5"},
        # INTSING の定義の形だけ (`d`)。TIGHTEN_INTBOUND は既定に入ったので付けない
        "isd": {"ENOMOTO_MIP_INTSING": "d"}}
