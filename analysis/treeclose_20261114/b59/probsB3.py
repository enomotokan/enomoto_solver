"""調べ B3 (treeclose 9.53 節): 分枝の選び方を HiGHS に寄せる鍵を 1 つずつ。kasai・csched008・sct2 × 打ち切り値つき 1 スレッド 600 秒
(B2 の t1 が基準)。HiGHS 1.15.1 の HighsPseudocost::getScore は mapScore(費用) + 1e-2 mapScore(衝突) + 1e-4 (mapScore(打ち切り率) + mapScore(推論))、
費用の正規化は max(1e-6, 平均^2)、SCIP の動的な重みはない。信頼度の閾値は両方 8 (mip_pscost_minreliable)。"""
from probsB import PROBS as _P
PROBS = [p for p in _P if p[0] in ("neos-3627168-kasai", "csched008", "sct2")]
CFGS = {
    "g1sw": ("1", {"ENOMOTO_MIP_GAP1": "usw"}),                      # 擬似費用の正規化の下限 1e-6・動的な重みなし (HiGHS の形)
    "cw": ("1", {"ENOMOTO_T_MIP_CONFLICT_WEIGHT": "0.01"}),          # 衝突の項の重み 0 → 0.01 (HiGHS)
    "hs": ("1", {"ENOMOTO_MIP_GAP1": "usw", "ENOMOTO_T_MIP_CONFLICT_WEIGHT": "0.01"}),  # 上の 2 つ (HiGHS の点数に近い)
    "la": ("1", {"ENOMOTO_T_MIP_SB_LOOKAHEAD": "1000"}),             # 強分岐の先読みの打ち切りをなくす (信頼できない候補を全部試す)
    "mr4": ("1", {"ENOMOTO_T_MIP_MINREL": "4"}),                     # 信頼度の閾値 8 → 4
    "mr16": ("1", {"ENOMOTO_T_MIP_MINREL": "16"}),                   # 8 → 16
}
