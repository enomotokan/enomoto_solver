"""根の下界 (analysis/rootbound_20261126) の試作 `ENOMOTO_MIP_PATHH_SLACK` (経路集約で、すき間のある行も使う) の P での比べ (2026-11-26、親の依頼) の設定。
既定 (def_t1・def_t4) と試作 (ps_t1・ps_t4) を 1・4 スレッドで。形は analysis/polishp_20261125/cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("ps", {"ENOMOTO_MIP_PATHH_SLACK": "1"}, True, True, "rootbound_20261126"),
}
CFGS = {"def_t1": {}, "def_t4": {}, "ps_t1": {"ENOMOTO_MIP_PATHH_SLACK": "1"}, "ps_t4": {"ENOMOTO_MIP_PATHH_SLACK": "1"}}
