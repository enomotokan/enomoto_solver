"""b58k (analysis/treeclose_20261114.md 9.20 節): 深さ優先寄りに関わる連続パラメータのベイズ最適化の探索空間。
すべて束 cand1116 (analysis/cand1116_20261115/cfg.py の CAND1116) + `ENOMOTO_MIP_NODESEL_BANDIT=4` (学習なしの交互) の上。
x は [0, 1]^5 に正規化した点。"""
import math

# (名前, 説明)
PARAMS = [
    ("s", "腕 1 (深さ優先寄り) を選ぶ区間の割合 ENOMOTO_T_MIP_NB4_SHARE: 0〜1"),
    ("w", "区間の長さ ENOMOTO_T_MIP_NB_WORK (LP の反復): 対数で 5e3〜1e5"),
    ("pq1", "腕 1 の潜りの打ち切りの割合 ENOMOTO_T_MIP_NB_ARM1_PQ: 0.25〜1"),
    ("sq1", "腕 1 の兄弟へ戻る下界の割合 ENOMOTO_T_MIP_NB_ARM1_SQ: 0.25〜1"),
    ("u", "腕 1 の潜りのノード数の上限 (u = 0 で既定 MAX 1000・DIV 10、u = 1 で MAX 1e9・DIV 1、対数で間を結ぶ)"),
]
DIM = len(PARAMS)
# 今の =4 の既定 (s 0.5、区間 2 万、腕 1 は d3 = PQ 1・SQ 1・ノード数の上限は既定)
BASELINE = [0.5, (math.log10(2e4) - math.log10(5e3)) / (math.log10(1e5) - math.log10(5e3)), 1.0, 1.0, 0.0]


def to_env(x):
    s, w, pq1, sq1, u = [min(1.0, max(0.0, float(v))) for v in x]
    work = int(round(10 ** (math.log10(5e3) + w * (math.log10(1e5) - math.log10(5e3)))))
    return {
        "ENOMOTO_MIP_NODESEL_BANDIT": "4",
        "ENOMOTO_T_MIP_NB4_SHARE": f"{s:.4f}",
        "ENOMOTO_T_MIP_NB_WORK": str(work),
        "ENOMOTO_T_MIP_NB_ARM1_PQ": f"{0.25 + 0.75 * pq1:.4f}",
        "ENOMOTO_T_MIP_NB_ARM1_SQ": f"{0.25 + 0.75 * sq1:.4f}",
        "ENOMOTO_T_MIP_NB_ARM1_NODES_MAX": str(int(round(10 ** (3 + 6 * u)))),
        "ENOMOTO_T_MIP_NB_ARM1_NODES_DIV": str(max(1, int(round(10 ** (1 - u))))),
    }
