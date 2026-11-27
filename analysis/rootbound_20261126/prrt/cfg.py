"""根の下界 (analysis/rootbound_20261126 の 6 節) の試作 `ENOMOTO_MIP_PRR_TIGHT` (プロービングが境界を締めたら、固定が 0 本でも前処理をかけ直して
またプロービングする、を繰り返す) の P での比べ (2026-11-27、親の依頼)。既定 (def_t1・def_t4) と試作 (pt_t1・pt_t4) を 1・4 スレッドで。
形は ../cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("pt", {"ENOMOTO_MIP_PRR_TIGHT": "1"}, True, True, "rootbound_20261126"),
}
CFGS = {"def_t1": {}, "def_t4": {}, "pt_t1": {"ENOMOTO_MIP_PRR_TIGHT": "1"}, "pt_t4": {"ENOMOTO_MIP_PRR_TIGHT": "1"}}
