"""群 1 (SCIP が数秒で決着する問題、analysis/vsscip_slow_20261118/g1/README.md) の設定: def = 既定 (環境変数なし)、
g1 = 双対の集約 (ENOMOTO_MIP_VUBAGG) + パッキングのオービトープを静的に (ENOMOTO_MIP_ORBPP)、vub・opp = それぞれ単独 (切り分け用)。"""
CFGS = {
    "def": {},
    "g1": {"ENOMOTO_MIP_VUBAGG": "1", "ENOMOTO_MIP_ORBPP": "1"},
    "vub": {"ENOMOTO_MIP_VUBAGG": "1"},
    "opp": {"ENOMOTO_MIP_ORBPP": "1"},
}
