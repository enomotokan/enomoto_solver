"""INC2 に仕事量の条件を足した形 (inc2w) の対の比べ (treeclose 9.67 節、2026-11-17)。
inc2w = `ENOMOTO_MIP_TREE_RESTART_INC2=1` (既定の `ENOMOTO_T_MIP_TREE_RESTART_INC2_WORKMULT` = 1: 木に入ってからの手間 (ticks) が根の段の手間以上のときだけ)。"""
CFGS = {"def": {}, "inc2w": {"ENOMOTO_MIP_TREE_RESTART_INC2": "1"}}
