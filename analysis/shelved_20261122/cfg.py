# 見送った試作の測り直しの設定 (analysis/shelved_20261122/README.md)。def は今の既定。
# ov: 期限の後だけ働く超過の直し 3 つ、fz2: 前処理の整数の丸めの余裕を許容誤差に揃える、ipmxm: 根の LP の内点法 + クロスオーバーの失敗の直し一式
CFGS = {
    "def": {},
    "ov": {"ENOMOTO_MIP_OVERRUN3": "ild"},
    "fz2": {"ENOMOTO_MIP_FZ2_FIX": "1"},
    "ipmxm": {"ENOMOTO_XO_MIP_CLEANUP_HINT": "1", "ENOMOTO_XO_IPM_RETRY_PC": "3", "ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST": "1", "ENOMOTO_IPM_KKT_AUG_RATIO": "4"},
}
