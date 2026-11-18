"""gate2 候補 8 の設定: def = 今の既定 (環境変数なし)、cc = CUTCONF だけ (比べ用)、cclg = CUTCONF + 長さのゲート
(作った制約の平均の長さ L で入れる割合 σ((25 − L)/5) を決める、analysis/gate2_20261117/cclg/README.md)。"""
CFGS = {
    "def": {},
    "cc": {"ENOMOTO_MIP_CUTCONF": "1"},
    "cclg": {"ENOMOTO_MIP_CUTCONF": "1", "ENOMOTO_MIP_CUTCONF_LGATE": "25,5,20"},
}
