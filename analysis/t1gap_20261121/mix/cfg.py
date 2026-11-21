"""t1gap の案 1・3 の試作の設定 (analysis/t1gap_20261121/README.md の 6 節)。名前が _t1 で終われば 1 スレッド、ほかは 4 スレッド (run.py の TH)。
mix: ENOMOTO_MIP_T1MIX (1 スレッドの根のカットのループの終わりに FJ + Local-MIP の組を本体で回す。4 スレッドでは何もしない)。
rens: ENOMOTO_MIP_T1RENS=1 (1 スレッドの外側の根の RENS の手間の期限を、並列の RENS と同じ残り時間まで延ばす。既存の試作、vsscip_slow_20261118/tree3)。"""
MIX = {"ENOMOTO_MIP_T1MIX": "1"}
RENS = {"ENOMOTO_MIP_T1RENS": "1"}
CFGS = {}
for suf in ("", "_t1"):
    CFGS["def" + suf] = {}
    CFGS["mix" + suf] = dict(MIX)
    CFGS["rens" + suf] = dict(RENS)
    CFGS["mixrens" + suf] = {**MIX, **RENS}
