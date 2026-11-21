"""primalgap の直し (ENOMOTO_MIP_SUBMIP_FEASTOL) の広い計測の設定。def: 今の既定 (直しあり)。sft0: 直しなし (以前の動き)。どれも 4 スレッド。"""
CFGS = {"def": {}, "sft0": {"ENOMOTO_MIP_SUBMIP_FEASTOL": "0"}}
