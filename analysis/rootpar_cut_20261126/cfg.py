"""根のカットの段の並列の試作 `ENOMOTO_MIP_ROOTPAR_CUT` (analysis/rootpar_cut_20261126) の P での比べ (2026-11-26、親の依頼) の設定。
既定 (def_t4) と試作 (rpc_t4) を 4 スレッドで (試作は 1 スレッドでは働かない)。形は analysis/polishp_20261125/cfg.py と同じ (summ.py をそのまま使う)。"""
P1 = {  # 番号: (名前, 環境変数, 1 スレッドで回すか, 4 スレッドで回すか, 記録)
    1: ("rpc", {"ENOMOTO_MIP_ROOTPAR_CUT": "1"}, False, True, "rootpar_cut_20261126"),
}
CFGS = {"def_t4": {}, "rpc_t4": {"ENOMOTO_MIP_ROOTPAR_CUT": "1"}}
