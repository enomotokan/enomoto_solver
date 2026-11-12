"""ROOTCAP の設定 (試作 ENOMOTO_MIP_ROOTCAP、analysis/rootcap_20261111.md)。"""
CFGS = {
    "def": {},                                   # 既定
    "rc": {"ENOMOTO_MIP_ROOTCAP": "hdw"},        # 根のヒューリスティクスの時間の上限 (h) + 裏のカットの待ちも同じ期限まで (w) + 根の LP が重い問題の密なカットの上限 (d)
    "rch": {"ENOMOTO_MIP_ROOTCAP": "h"},         # h だけ
}
