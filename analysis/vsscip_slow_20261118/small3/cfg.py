"""群 3 (小さい問題の手間) の設定 (analysis/vsscip_slow_20261118/small3/README.md)。def = 既定 (環境変数なし)。
s3g = 外側の根で暫定解と下界の相対の間隙が 0.002 未満なら重い根のヒューリスティクス (RENS・RINS・ダイビングなど) を回さない。
er1 = プロービングで見つけた同値な 0-1 列の代入による前処理のかけ直しを 1 回まで (既存の調整、既定 2)。"""
CFGS = {
    "def": {},
    "s3g": {"ENOMOTO_MIP_S3GAP": "0.002"},
    "er1": {"ENOMOTO_T_MIP_PROBE_EQUIV_ROUNDS": "1"},
}
