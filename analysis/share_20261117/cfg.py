"""共有を切る・増やす診断 (treeclose 9.75 節の 3)。どれも 4 スレッド、既定に診断の環境変数を 1 つずつ足す。
M = 重い 2 問 (600 秒)、P = 58 問で節点の水増しが大きい 6 問 (120 秒)。種 1・2。1 スレッドの比べ先は parprof_20261117。"""
CFG = {
    "def": {},
    "pc0": {"ENOMOTO_T_MIP_TREE_PC_SYNC": "0"},          # 擬似費用を合わせない
    "pc1": {"ENOMOTO_T_MIP_TREE_PC_SYNC": "1"},          # 擬似費用を毎ノード合わせる (既定 16)
    "noproof": {"ENOMOTO_MIP_TREE_NO_SHARED_PROOFS": "1"},  # 双対証明・衝突を共有しない
    "nobnd": {"ENOMOTO_MIP_TREE_NO_SHARED_BOUNDS": "1"},    # 大域の境界を共有しない
    "sync8": {"ENOMOTO_T_MIP_TREE_SYNC_EVERY": "8"},     # 境界・証明・衝突の取り込みを 8 ノードおき (既定 64)
    "cuts": {"ENOMOTO_MIP_TREE_SHARE_CUTS": "1"},         # カットのプールを共有する
    "atonce": {"ENOMOTO_MIP_TREE_SHARE_PROOFS_AT_ONCE": "1", "ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE": "1"},  # 証明・衝突をすぐ出す
    "nofix": {"ENOMOTO_MIP_TREE_NO_SHARED_FIX": "1"},     # 共有の最良値での根の被約費用の固定のやり直しをしない
    "nobal": {"ENOMOTO_MIP_TREE_NO_BALANCE": "1"},        # 32 ノードおきの待ち行列の釣り合わせをしない
}
M = ["neos-3627168-kasai", "csched008"]
P = ["neos5", "noswot", "mas76", "timtab1", "neos-911970", "roll3000"]
SEEDS = [1, 2]
TL = {"M": 600, "P": 120}
