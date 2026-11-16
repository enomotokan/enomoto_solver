"""調べ B2 (treeclose 9.52 節): 根が同じなのに閉じない (csched008・sct2) と最後の詰めが遅い (kasai) 3 問で、最適値を打ち切り値に与えた
証明だけの比べ (b59 の ours_cut と同じ形) を 1 スレッド (HiGHS と同じく木 1 本) で回し、木の中の部品を 1 つずつ切り替える。"""
from probsB import PROBS as _P
PROBS = [p for p in _P if p[0] in ("neos-3627168-kasai", "csched008", "sct2")]
# 設定: 名前 → (スレッド数, 環境変数)
CFGS = {
    "t1": ("1", {}),
    "t1_dfs0": ("1", {"ENOMOTO_MIP_DFSBB": "0"}),                 # HiGHS 型の木の進め方を切る (0.1.7 の潜り方)
    "t1_ncut": ("1", {"ENOMOTO_MIP_NODE_CUTS": "1"}),             # ノードで毎回分離する (既定は 10 段ごと)
    "t1_noconf": ("1", {"ENOMOTO_MIP_NO_CONFLICTS": "1"}),        # 衝突解析を切る
    "t1_norst": ("1", {"ENOMOTO_MIP_NO_RESTART": "1"}),          # 再スタート (根・木の中) を切る
    "t4": ("4", {}),                                              # 4 スレッド (b59 の ours_cut と同じ、揺れの確かめ)
}
