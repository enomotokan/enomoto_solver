"""run2.py・pair.py の設定 (束 cand1114 に足す環境変数) と変種。"""
CFGS = {
    "base": {},
    "oi": {"ENOMOTO_MIP_ORDINV": "edpq"},            # 型 D: 前処理を列の鍵の順に
    "oc": {"ENOMOTO_MIP_ORDINV": "c"},               # 型 D: 求解の入口で行・列を鍵の順に並べ直す (正準化)
    "oic": {"ENOMOTO_MIP_ORDINV": "edpqc"},          # 型 D: 正準化 + 前処理の手当て
    "oc1": {"ENOMOTO_MIP_ORDINV": "c", "ENOMOTO_T_MIP_ORDINV_SALT": "1"},  # 正準化の塩 1 (別の正準な順、揺れを測る用)
    "rbp": {"ENOMOTO_MIP_RSTROB": "+"},              # 型 C: 再スタートの判定の帯の中は再スタートする
    "rbm": {"ENOMOTO_MIP_RSTROB": "-"},              # 型 C: 帯の中は再スタートしない
    "race": {"ENOMOTO_MIP_TREE_RACING": "1"},        # 4: 木の競争 (既存の試作)
    "prace": {"ENOMOTO_MIP_PERMRACE": "1"},          # 4: 元の問題と並べ替えた問題を半分ずつのスレッドで競わせる
}
VARIANTS = [(0, 1), (1, 1), (2, 1), (3, 1), (0, 2), (0, 3)]
