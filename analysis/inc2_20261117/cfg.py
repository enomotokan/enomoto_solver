"""木の中の再スタートの試作 `ENOMOTO_MIP_TREE_RESTART_INC2` の対の比べ (treeclose 9.63 節、2026-11-17)。
def = 既定、inc2 = 暫定解が良くなり根の被約費用で新たに固まった整数列が、根の始めに自由だった整数列の 10% 以上なら木の中で再スタート
(既定の刈った重みの判定に足す。閾値 `ENOMOTO_T_MIP_TREE_RESTART_INC2_FRAC`)。"""
CFGS = {"def": {}, "inc2": {"ENOMOTO_MIP_TREE_RESTART_INC2": "1"}}
