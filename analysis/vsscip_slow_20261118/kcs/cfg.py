"""群 2 kcs の設定: def = 既定、kcs = `ENOMOTO_MIP_KEEP_COSTED_SINGLETON=1` (整数最適化の前処理で費用のついた連続の列シングルトンを消さない。
analysis/vsscip_slow_20261118/tree2/README.md 10 節、neos-5107597-kakapo の根 1.11 → 538.9)。"""
CFGS = {
    "def": {},
    "kcs": {"ENOMOTO_MIP_KEEP_COSTED_SINGLETON": "1"},
    # kcsc = 等式のほかの列がすべて連続で、上下限の片方が無限の列だけ残す (markshare_4_0・timtab1 の害を除く。kcs/README.md)
    "kcsc": {"ENOMOTO_MIP_KEEP_COSTED_SINGLETON": "c"},
}
