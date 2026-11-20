#!/bin/bash
# orbital fixing + LexRed (SYMDYN=f) の直しの確かめ: 対称性の fuzz を 直しあり (既定) と 直しなし (ENOMOTO_MIP_OF_ANY_FIXED=1) で回す。
# リポジトリの根から: bash analysis/wrong_widden_20261117/symfuzz.sh。結果は analysis/wrong_widden_20261117/symfuzz/*.txt
cd "$(dirname "$0")/../.."
O=analysis/wrong_widden_20261117/symfuzz
H="ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_NO_CUTS=1 ENOMOTO_MIP_NO_RESTART=1"
for v in new old; do
  X=""; [ $v = old ] && X="ENOMOTO_MIP_OF_ANY_FIXED=1"
  for sc in 1 2; do
    env ENOMOTO_MIP_REL_GAP=1e-9 $X $H python analysis/symdyn_20261110/fuzz_symdyn.py 200 0 $sc 2>/dev/null | grep -E "^bad|^seed|BAD" > $O/symdyn_hard_s${sc}_$v.txt
    env ENOMOTO_MIP_REL_GAP=1e-9 $X $H ENOMOTO_MIP_THREADS=4 python analysis/symdyn_20261110/fuzz_symdyn.py 100 2000 $sc 2>/dev/null | grep -E "^bad|^seed|BAD" > $O/symdyn_hard4t_s${sc}_$v.txt
  done
  env ENOMOTO_MIP_REL_GAP=1e-9 $X python scripts/mip_fuzz_symmetric.py 150 0 1 2>/dev/null | grep -E "^bad|^seed|BAD" > $O/symm_s1_$v.txt
  env ENOMOTO_MIP_REL_GAP=1e-9 $X ENOMOTO_MIP_NO_ORBITOPE=1 python scripts/mip_fuzz_symmetric.py 60 0 2 2>/dev/null | grep -E "^bad|^seed|BAD" > $O/symm_noorb_s2_$v.txt
  env ENOMOTO_MIP_REL_GAP=1e-9 $X ENOMOTO_MIP_NO_CUTS=1 python analysis/sym_of_20261028/fuzz_of.py 100 0 2 2>/dev/null | grep -E "^bad|^seed|BAD" > $O/of_nocut_s2_$v.txt
done
for f in $O/*.txt; do echo "$(basename $f): $(tail -1 $f)"; done
