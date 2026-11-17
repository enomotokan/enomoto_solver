"""伝播の締めすぎの直しの確かめ。def = 既定 (ENOMOTO_T_PROP_RELTOL=0)、rt = ENOMOTO_T_PROP_RELTOL=1e-10 (不等式の伝播で、有限の境界は締めの幅が
1e-10 × (1 + |境界|) を超えるときだけ締める)、rtc = ENOMOTO_T_PROP_RELTOL_CONT=1e-10 (同じ閾値を整数最適化の前処理の連続列だけにかける)。"""
CFGS = {"def": {}, "rt": {"ENOMOTO_T_PROP_RELTOL": "1e-10"}, "rtc": {"ENOMOTO_T_PROP_RELTOL_CONT": "1e-10"}}
