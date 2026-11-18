"""LPINF の対の比べの設定: def = 既定 (際どいノードの LP の実行不能を信用しない直し、558d7a4ce)、old = 直しなし (ENOMOTO_MIP_NO_LPINF_CHECK=1)。"""
CFGS = {
    "def": {},
    "old": {"ENOMOTO_MIP_NO_LPINF_CHECK": "1"},
}
