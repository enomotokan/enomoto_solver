#!/bin/bash
# timtab1: HiGHS と ENOMOTO の比較 (200 秒・60 秒・打ち切り値つき・対称性の形)
cd "$(dirname "$0")"
C=764772.5
for r in 1 2; do
  ./run.sh highs timtab1 200 h200
  ./run.sh eno timtab1 200 def200
  ./run.sh eno timtab1 200 skipfixed200 ENOMOTO_MIP_SYM_SKIP_FIXED=1
  ./run.sh highs timtab1 200 hcut200 objective_bound=$C
  ./run.sh eno timtab1 200 defcut200 ENOMOTO_MIP_CUTOFF=$C
  ./run.sh eno timtab1 200 skipfixedcut200 ENOMOTO_MIP_SYM_SKIP_FIXED=1 ENOMOTO_MIP_CUTOFF=$C
  ./run.sh highs timtab1 60 h60
  ./run.sh eno timtab1 60 def60
done
./run.sh eno timtab1 200 nosym200 ENOMOTO_MIP_NO_SYMMETRY=1
./run.sh eno timtab1 200 orbonly200 ENOMOTO_MIP_SYM_DETECT_ONLY=1
echo BATCH1_DONE
