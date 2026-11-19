"""SSTX の設定 (analysis/sstx_20261119/README.md)。
- def: このコミットの既定 (orbital fixing)。
- sst: orbital fixing の代わりに Schreier-Sims の表のカット (試作 ENOMOTO_MIP_SYM_SST=1、analysis/hang_20261117)。
- sstx: sst に加え、SST の行に反するヒューリスティクスの解を SST の段の群で写して暫定解にする (試作 ENOMOTO_MIP_SYM_SST=x)。"""
CFGS = {
    "def": {},
    "sst": {"ENOMOTO_MIP_SYM_SST": "1"},
    "sstx": {"ENOMOTO_MIP_SYM_SST": "x"},
}
