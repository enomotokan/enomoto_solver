"""P2-SCIP-M の計測の設定 (analysis/slowm_20261119)。def: 今の既定。fix: カットの締め付けの誤答の直し (ENOMOTO_MIP_TIGHTEN_INTBOUND=1)。
fixis: 直し + 試作 ENOMOTO_MIP_INTSING=1 (等式 1 本にだけ現れる整数列の消去)。st5: 根のカットの整数刻みの停滞の打ち切りを 3 → 5 巡
(ENOMOTO_T_MIP_CUT_INTOBJ_STALL=5、neos-3381206-awhea 1 スレッド 30.7 → 3.5 秒)。どれも 4 スレッド。"""
CFGS = {"def": {}, "fix": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1"}, "fixis": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1", "ENOMOTO_MIP_INTSING": "1"},
        "st5": {"ENOMOTO_T_MIP_CUT_INTOBJ_STALL": "5"},
        # INTSING の定義の形だけ (`d`)。M の fixis と同じく TIGHTEN_INTBOUND も付ける (既定に入った後のビルドでは同じ)
        "isd": {"ENOMOTO_MIP_TIGHTEN_INTBOUND": "1", "ENOMOTO_MIP_INTSING": "d"},
        # INTSING から「数える一般整数列」(費用あり・幅 > 1・行の他の係数がすべて逆符号) を外す形 (`g`)
        "isg": {"ENOMOTO_MIP_INTSING": "1g"},
        # g に加えて同符号の 0-1 列 (行の他の係数がすべて同符号) も消さない形 (`s`)
        "isgs": {"ENOMOTO_MIP_INTSING": "1gs"},
        # g に加えて、支配される 0-1 列の固定を元の問題に先にかける形 (`o`、README §13.4)。P・M で 1g と前処理が変わるのは air03 だけ
        "isgo": {"ENOMOTO_MIP_INTSING": "1go"},
        # 双対証明・カットのプールの伝播で締めた変更も衝突解析で遡る (README §17、neos859080 は 6 種で 23〜58 → 8〜15 秒)
        "pe": {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1"}}
