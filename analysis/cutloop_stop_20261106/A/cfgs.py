"""cutloop_stop A の設定 (環境変数の組)。"""
CFGS = {
    "def": {},
    "c": {"ENOMOTO_MIP_CUT0_RETRY": "c"},
    "c1": {"ENOMOTO_MIP_CUT0_RETRY": "c"},  # 切り替えの渡し方を直す前の版 (コミット a0d55c85 の c)。名前だけ分けた
    "r": {"ENOMOTO_MIP_CUT0_RETRY": "1"},
    "mv": {"ENOMOTO_MIP_HO4_CUT0": "moved"},
}
