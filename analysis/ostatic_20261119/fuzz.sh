#!/bin/bash
# 試作 ostr (ENOMOTO_MIP_ORBITOPE_STATIC=r) と orr (ENOMOTO_MIP_ORBITOPE_REASON=1、ORBPP を小さいオービトープでも働かせる MIN_COLS=0・MIN_ROWS=1) の誤答の確かめ。
# リポジトリの根で pip install -e . の後に: bash analysis/ostatic_20261119/fuzz.sh  (ログは analysis/ostatic_20261119/fuzz/、4 本ずつ並べる)
cd "$(dirname "$0")/../.."
O=analysis/ostatic_20261119/fuzz
run() { name=$1; shift; env "$@" > $O/$name.log 2>&1; echo "$name rc=$? $(grep -i -E 'bad|mismatch|wrong' $O/$name.log | tail -1)"; }
OSTR="ENOMOTO_MIP_ORBITOPE_STATIC=r"
ORR="ENOMOTO_MIP_ORBITOPE_REASON=1 ENOMOTO_T_MIP_ORBPP_MIN_COLS=0 ENOMOTO_T_MIP_ORBPP_MIN_ROWS=1"
run ostr_sym1 $OSTR python scripts/mip_fuzz_symmetric.py 300 0 1 &
run ostr_sym2 $OSTR python scripts/mip_fuzz_symmetric.py 300 0 2 &
run orr_sym1 $ORR python scripts/mip_fuzz_symmetric.py 300 0 1 &
run orr_sym2 $ORR python scripts/mip_fuzz_symmetric.py 300 0 2 &
wait
run ostr_highs $OSTR python scripts/mip_fuzz_vs_highs.py 300 &
run orr_highs $ORR python scripts/mip_fuzz_vs_highs.py 300 &
run ostr_of $OSTR python analysis/sym_of_20261028/fuzz_of.py 1000 0 &
run orr_of $ORR python analysis/sym_of_20261028/fuzz_of.py 1000 0 &
wait
