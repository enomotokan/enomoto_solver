"""調べ B の問題と最適値 (最小化、cand1118 の M と HiGHS の最適)。整数の目的値なら打ち切り値は最適値 − 0.5"""
PROBS = [("neos-3627168-kasai", 988585.62, False), ("csched008", 173.0, True), ("unitcal_7", 19635558.2440195, False),
         ("sct2", -230.98916228571431, False), ("net12", 214.0, True), ("supportcase33", -345.0, True)]
CFGS = ["ours", "ours_cut", "highs", "highs_cut"]
