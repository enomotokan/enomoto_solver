"""ROOTCAP の設定 (試作 ENOMOTO_MIP_ROOTCAP、analysis/rootcap_20261111.md)。"""
CFGS = {
    "def": {},                                   # 既定
    "rc": {"ENOMOTO_MIP_ROOTCAP": "hd"},         # 根のヒューリスティクスの時間の上限 (h) + 根の LP が重い問題の密なカットの上限 (d)。w (裏のカットの待ちを切る) は対象の問題で下界を下げたので外した
    "rch": {"ENOMOTO_MIP_ROOTCAP": "h"},         # h だけ
}
