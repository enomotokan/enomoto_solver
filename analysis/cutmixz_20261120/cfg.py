"""CUTMIX=z の広い計測の設定 (analysis/cutmixz_20261120/README.md)。どちらも 4 スレッド。
- def: このコミットの既定 (zero-half は切ってある、`ENOMOTO_MIP_NO_ZEROHALF` の既定 `1`)。
- z: `ENOMOTO_MIP_CUTMIX=z`。全部整数で係数が絶対値 2 以下の行の割合 (intsmall、根のカットのループの初めに数える) が 0.5 以上の問題でだけ zero-half を使う。
"""
CFGS = {"def": {}, "z": {"ENOMOTO_MIP_CUTMIX": "z"}}
