"""scip_rens の設定: def = 今の既定、lfc = 試作 ENOMOTO_MIP_LPFACE=cf (LP face のサブ MIP に目的値の行 ≤ 根の LP 値を足し、根の被約費用のサブ MIP より先に回す。README)。"""
CFGS = {
    "def": {},
    "lfc": {"ENOMOTO_MIP_LPFACE": "cf"},
    # r: 行のサブ MIP が解なしなら、行を外した今の LP face を残りのノードで解き直す (README 9 節)
    "cfr": {"ENOMOTO_MIP_LPFACE": "cfr"},
}
