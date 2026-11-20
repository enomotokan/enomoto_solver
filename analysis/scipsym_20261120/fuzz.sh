#!/bin/bash
# 試作 ENOMOTO_MIP_SYM_PPLEX=1 の誤答の確かめ。リポジトリの根から: bash analysis/scipsym_20261120/fuzz.sh
# 結果は fuzz/<名前>.txt (最後の行が bad の数)、働いた回数 (PPLEX が成分を格上げした求解の数) は fuzz/<名前>.cov。標準エラーのログは残さない。
set -u
cd "$(dirname "$0")/../.."
F=analysis/scipsym_20261120/fuzz; mkdir -p $F
run() { name=$1; shift; env ENOMOTO_MIP_SYM_PPLEX=1 ENOMOTO_MIP_LOG=1 "$@" 2> $F/$name.err | grep -E "^(seed|bad)" > $F/$name.txt; echo "$(grep -c 'PPLEX: [1-9]' $F/$name.err) upgraded / $(grep -c 'PPLEX:' $F/$name.err) runs with symmetry" > $F/$name.cov; rm -f $F/$name.err; echo "$name: $(tail -1 $F/$name.txt) / $(cat $F/$name.cov)"; }
run pplex_a python3 analysis/scipsym_20261120/fuzz_pplex.py 600 0
run pplex_t1 env ENOMOTO_MIP_THREADS=1 python3 analysis/scipsym_20261120/fuzz_pplex.py 600 1000
run pplex_t4 env ENOMOTO_MIP_THREADS=4 python3 analysis/scipsym_20261120/fuzz_pplex.py 600 2000
run pplex_noorb env ENOMOTO_MIP_NO_ORBITOPE=1 python3 analysis/scipsym_20261120/fuzz_pplex.py 600 3000
run sym1 python3 scripts/mip_fuzz_symmetric.py 300 0 1
run sym2_noorb env ENOMOTO_MIP_NO_ORBITOPE=1 python3 scripts/mip_fuzz_symmetric.py 300 0 2
run sym1_noorb env ENOMOTO_MIP_NO_ORBITOPE=1 python3 scripts/mip_fuzz_symmetric.py 300 0 1
run vs_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
