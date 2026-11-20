"""並列の木の探索の順の試作 (`ENOMOTO_MIP_PARORD`、treeclose 9.78 節)。どれも 4 スレッド。
a = 作業者 0 だけ大域の下界最小、ae = それに加えてほかは推定値だけ (HiGHS 1.15.1 と同じ分け方)、
cg = 番号 3 のスレッドを暫定解がない間 (c) と相対の間隙が 5% を超える間 (g) ヒューリスティクス役に回す、
p3 = 既存の試作 POOL3_BEST (0.05 秒おきに全体の最小の下界のノードを必ず処理する、pool_bound3_20261104.md) の比べ先。"""
CFG = {
    "def": {},
    "a": {"ENOMOTO_MIP_PARORD": "a"},
    "ae": {"ENOMOTO_MIP_PARORD": "ae"},
    "cg": {"ENOMOTO_MIP_PARORD": "cg"},
    "p3": {"ENOMOTO_MIP_POOL3_BEST": "0.05"},
}
# 対の計測 (P = 58 問 60 秒 種 1〜3、M = 重い 4 問 600 秒 種 1・2) に回す設定。手元確認 (L) で既定より速めだった cg・p3 と、csched008 を種 1・2 とも解いた a。
PM = ["def", "a", "cg", "p3"]
