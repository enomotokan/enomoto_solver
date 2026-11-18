"""群 2 domrev の設定: def = 既定 (環境変数なし)、drev = 支配される列の固定を、支配される側から候補を探す形で使う
(ENOMOTO_MIP_DOMCOL + ENOMOTO_MIP_DOMCOL_PRED + ENOMOTO_MIP_DOMCOL_REV。analysis/vsscip_slow_20261118/tree2/README.md)。"""
CFGS = {
    "def": {},
    "drev": {"ENOMOTO_MIP_DOMCOL": "1", "ENOMOTO_MIP_DOMCOL_PRED": "1", "ENOMOTO_MIP_DOMCOL_REV": "1"},
}
