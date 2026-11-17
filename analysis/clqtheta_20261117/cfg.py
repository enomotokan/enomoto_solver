"""CLQ_LAZY の外す条件のしきい値 θ の掃引 (treeclose 9.83 節、2026-11-17)。外すのは根の LP 解 (木に入るとき) で文字の和 < θ (かつスラックが基底) の
クリーク型の不等式の行。`ENOMOTO_T_MIP_CLQ_LAZY_TOL` = 1 − θ (外す条件は 1 − 和 > TOL)。def は試作なし。"""
THETAS = {"t0": 1e-6, "t01": 0.1, "t03": 0.3, "t05": 0.5, "t07": 0.7, "t09": 0.9, "t1": 1 - 1e-6}
CFGS = {"def": {}}
for k, th in THETAS.items():
    CFGS[k] = {"ENOMOTO_MIP_CLQ_LAZY": "1", "ENOMOTO_T_MIP_CLQ_LAZY_TOL": repr(1.0 - th)}
# P: 58 問のうち根の LP で余裕のあるクリーク型の行がある問題 (clqrows_20261117/res_drop.jsonl)、60 秒。M: 重い問題、600 秒
P = ["exp-1-500-5-5", "h80x6320d", "mzzv42z", "neos-827175", "neos-860300", "neos-960392", "neos8", "physiciansched6-2", "roll3000", "misc07",
     "dcmulti", "gt2", "10teams", "beavma", "blp-ar98", "neos-1456979"]
M = ["mzzv11", "neos-933966", "supportcase33", "neos-3216931-puriri"]
SEEDS = [1, 2]
TL = {"P": 60, "M": 600}
