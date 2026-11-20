"""群 2 domrev の設定: def = 既定 (環境変数なし)、drev = 支配される列の固定を、支配される側から候補を探す形で使う
(ENOMOTO_MIP_DOMCOL + ENOMOTO_MIP_DOMCOL_PRED + ENOMOTO_MIP_DOMCOL_REV。analysis/vsscip_slow_20261118/tree2/README.md)。"""
CFGS = {
    "def": {},
    "drev": {"ENOMOTO_MIP_DOMCOL": "1", "ENOMOTO_MIP_DOMCOL_PRED": "1", "ENOMOTO_MIP_DOMCOL_REV": "1"},
    # drevnp = drev + 固定の後のかけ直しでプロービングをやり直さない (tree2/README.md 4 節、air03 の害)
    "drevnp": {"ENOMOTO_MIP_DOMCOL": "1", "ENOMOTO_MIP_DOMCOL_PRED": "1", "ENOMOTO_MIP_DOMCOL_REV": "1", "ENOMOTO_MIP_DOMCOL_NOPROBE2": "1"},
    # drevmf = drevnp + 固定が自由な列の 10% に満たなければ捨てる (tree2/README.md 4 節、nw04・nu25-pr12・fiber の害)
    "drevmf": {"ENOMOTO_MIP_DOMCOL": "1", "ENOMOTO_MIP_DOMCOL_PRED": "1", "ENOMOTO_MIP_DOMCOL_REV": "1", "ENOMOTO_MIP_DOMCOL_NOPROBE2": "1",
               "ENOMOTO_T_MIP_DOMCOL_MIN_FRAC": "0.1"},
    # pst = 根の経路集約の始点の上限を外す (既定 1000。tree2/README.md 5 節、istanbul-no-cutoff で根 71.6 → 118.2)
    "pst": {"ENOMOTO_T_MIP_PATH_STARTS": "1000000"},
}
