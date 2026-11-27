# rootpar_ph の P の比べの設定 (README.md)。def は既定、w は ENOMOTO_MIP_ROOTPAR_PH=w (研磨の LP に整数を固定した LP と同じ時間の上限)、
# b は ENOMOTO_MIP_ROOTPAR_PH=b (根の研磨を別スレッドで回す、2 スレッド以上だけ)。名前が _t1 で終われば 1 スレッド
CFGS = {"def": {}, "w": {"ENOMOTO_MIP_ROOTPAR_PH": "w"}, "b": {"ENOMOTO_MIP_ROOTPAR_PH": "b"},
        "def_t1": {}, "w_t1": {"ENOMOTO_MIP_ROOTPAR_PH": "w"},
        # README 5 節: eg = POLISH_WARM=e + ROOTPAR_PH=g (木の中は漏れの見積りだけ、返す前に期限なしで 1 回研磨)、egj = それに j (研磨の LP を同時実行で速く)
        "eg": {"ENOMOTO_MIP_POLISH_WARM": "e", "ENOMOTO_MIP_ROOTPAR_PH": "g"}, "egj": {"ENOMOTO_MIP_POLISH_WARM": "e", "ENOMOTO_MIP_ROOTPAR_PH": "gj"},
        "eg_t1": {"ENOMOTO_MIP_POLISH_WARM": "e", "ENOMOTO_MIP_ROOTPAR_PH": "g"}, "egj_t1": {"ENOMOTO_MIP_POLISH_WARM": "e", "ENOMOTO_MIP_ROOTPAR_PH": "gj"}}
