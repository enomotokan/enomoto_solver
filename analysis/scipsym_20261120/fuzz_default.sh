#!/bin/bash
# 既定化したビルド (環境変数なし = PPLEX が既定) での誤答の確かめ。リポジトリの根から: bash analysis/scipsym_20261120/fuzz_default.sh
# 結果は fuzz/def_<名前>.txt・.cov (fuzz.sh と同じ形)。
set -u
cd "$(dirname "$0")/../.."
F=analysis/scipsym_20261120/fuzz; mkdir -p $F
run() { name=def_$1; shift; env ENOMOTO_MIP_LOG=1 "$@" 2> $F/$name.err | grep -E "^(seed|bad)" > $F/$name.txt; echo "$(grep -c 'PPLEX: [1-9]' $F/$name.err) upgraded / $(grep -c 'PPLEX:' $F/$name.err) runs with symmetry" > $F/$name.cov; rm -f $F/$name.err; echo "$name: $(tail -1 $F/$name.txt) / $(cat $F/$name.cov)"; }
run pplex python3 analysis/scipsym_20261120/fuzz_pplex.py 600 5000
run pplex_t1 env ENOMOTO_MIP_THREADS=1 python3 analysis/scipsym_20261120/fuzz_pplex.py 600 6000
run sym1 python3 scripts/mip_fuzz_symmetric.py 300 300 1
run sym2_noorb env ENOMOTO_MIP_NO_ORBITOPE=1 python3 scripts/mip_fuzz_symmetric.py 300 300 2
run vs_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
