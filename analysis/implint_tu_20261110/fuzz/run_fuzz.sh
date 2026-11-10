#!/bin/bash
# 誤答の検査 (低い優先度・1 スレッド。wheel は tu2v5 = e 入り)。結果は fuzz/*.out
cd "$(dirname "$0")"
WT=$(cd ../../.. && pwd)
export PYTHONPATH=/home/user/pkgs/tu2v5 ENOMOTO_MIP_THREADS=1
for v in baes bes 1 be; do
  ENOMOTO_MIP_IMPLINT_TU2=$v nice -n 19 python3 ../fuzz_tu.py 3000 0 > fuzz_tu_$v.out 2>&1
done
nice -n 19 python3 ../fuzz_tu.py 3000 0 > fuzz_tu_def.out 2>&1
for v in baes bes; do
  (cd "$WT/benchmarks/equiv2_bug_20261022" && ENOMOTO_MIP_IMPLINT_TU2=$v nice -n 19 python3 fuzz_tiny.py 400 0) > fuzz_tiny_$v.out 2>&1
  (cd "$WT" && ENOMOTO_MIP_IMPLINT_TU2=$v ENOMOTO_MIP_REL_GAP=1e-9 nice -n 19 python3 scripts/mip_fuzz_vs_highs.py 300) > fuzz_vs_highs_$v.out 2>&1
done
(cd "$WT/benchmarks/equiv2_bug_20261022" && nice -n 19 python3 fuzz_tiny.py 400 0) > fuzz_tiny_def.out 2>&1
(cd "$WT" && ENOMOTO_MIP_REL_GAP=1e-9 nice -n 19 python3 scripts/mip_fuzz_vs_highs.py 300) > fuzz_vs_highs_def.out 2>&1
echo FUZZ_DONE
