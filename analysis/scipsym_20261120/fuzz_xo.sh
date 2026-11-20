#!/bin/bash
# 試作 ENOMOTO_XO_BUMP_LU_RATIO=1 の誤答の確かめ。リポジトリの根から: bash analysis/scipsym_20261120/fuzz_xo.sh → fuzz/xo_*.txt
set -u
cd "$(dirname "$0")/../.."
F=analysis/scipsym_20261120/fuzz; mkdir -p $F
run() { name=xo_$1; shift; env ENOMOTO_XO_BUMP_LU_RATIO=1 "$@" 2>/dev/null | grep -E "^(seed|bad|BAD|mismatch)|bad" | tail -20 > $F/$name.txt; echo "$name: $(tail -1 $F/$name.txt)"; }
run vs_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run vs_highs_t1 env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300 1000
run lp_vs_highs python3 scripts/lp_fuzz_vs_highs.py 300
run pplex_t1 env ENOMOTO_MIP_THREADS=1 python3 analysis/scipsym_20261120/fuzz_pplex.py 300 7000
run sym1 python3 scripts/mip_fuzz_symmetric.py 300 600 1
