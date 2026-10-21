#!/bin/bash
cd /home/user/enomoto_solver
for r in 5 6; do for v in "def X=1" "A3 ENOMOTO_MIP_RESTORE_READD=3"; do set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_co_r$r neos-2978193-inde 120 ENOMOTO_MIP_CUTOFF=-2.38806169 "$@"; done; done
for r in 1 2; do
for p in misc07 mik-250-20-75-4 10teams neos-860300 binkar10_1 neos5 beavma qiu air03 rout; do
for v in "def X=1" "A3 ENOMOTO_MIP_RESTORE_READD=3"; do set -- $v; tag=$1; shift
  runs_nr/ab.sh side_${tag}_r$r $p 60 "$@"
done; done; done
echo BATCH3_DONE
