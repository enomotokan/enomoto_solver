"""既定の誤答の再現 (analysis/hnfpre_20261107.md §9.6): scripts/mip_fuzz_vs_highs.py の gen(3470) を HiGHS と ENOMOTO で解く。
ENOMOTO_MIP_PROBE_EQUIV2=0 を付けると正しくなる。使い方: python analysis/hnfpre_20261107/repro_seed3470.py (リポジトリの根で)"""
import sys
src=open('scripts/mip_fuzz_vs_highs.py').read().split("\nN=int(")[0]; ns={}; exec(src, ns)
lp=ns['gen'](3470); print(ns['highs'](lp)); r=ns['ours'](lp); print(r[0], r[1])
