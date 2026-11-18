"""群 2 cmz の設定: def = 既定 (環境変数なし)、cmz = `ENOMOTO_MIP_CUTMIX=z` (全部整数で係数が小さい行の割合が
`ENOMOTO_T_CUTMIX_ZH_MIN` (0.5) 以上の問題でだけ zero-half を使う。analysis/vsscip_slow_20261118/tree2/README.md 7 節)。"""
CFGS = {
    "def": {},
    "cmz": {"ENOMOTO_MIP_CUTMIX": "z"},
}
