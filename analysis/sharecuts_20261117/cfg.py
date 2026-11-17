"""カットのプールの共有 (sc) と待ち行列の釣り合わせなし (nb) の対の比べ (treeclose 9.77 節、2026-11-17)。
sc = `ENOMOTO_MIP_TREE_SHARE_CUTS=1`、nb = `ENOMOTO_MIP_TREE_NO_BALANCE=1`。"""
CFGS = {"def": {}, "sc": {"ENOMOTO_MIP_TREE_SHARE_CUTS": "1"}, "nb": {"ENOMOTO_MIP_TREE_NO_BALANCE": "1"}}
