"""puriri_cutlp の設定 (環境変数の組)。"""
CFGS = {
    "def": {},
    "h1": {"ENOMOTO_MIP_CUT_HARRIS2": "1"},
    "ht": {"ENOMOTO_MIP_CUT_HARRIS2": "t"},
    "hr": {"ENOMOTO_MIP_CUT_HARRIS2": "r"},
}
CFGS.update({
    "mv": {"ENOMOTO_MIP_HO4_CUT0": "moved"},
    "h1mv": {"ENOMOTO_MIP_CUT_HARRIS2": "1", "ENOMOTO_MIP_HO4_CUT0": "moved"},
    "htmv": {"ENOMOTO_MIP_CUT_HARRIS2": "t", "ENOMOTO_MIP_HO4_CUT0": "moved"},
})
