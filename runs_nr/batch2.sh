#!/bin/bash
cd /home/user/enomoto_solver
for r in 3 4; do for v in "def X=1" "P ENOMOTO_MIP_RESTORE_READD=2"; do set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_co_r$r neos-2978193-inde 120 ENOMOTO_MIP_CUTOFF=-2.38806169 "$@"; done; done
for r in 1 2; do for v in "def X=1" "P ENOMOTO_MIP_RESTORE_READD=2" "A3 ENOMOTO_MIP_RESTORE_READD=3"; do set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_co_r$r mc11 90 ENOMOTO_MIP_CUTOFF=11689.5 "$@"; done; done
for r in 1 2; do for v in "def X=1" "P ENOMOTO_MIP_RESTORE_READD=2" "A3 ENOMOTO_MIP_RESTORE_READD=3"; do set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_r$r neos-911970 60 "$@"; done; done
echo BATCH2_DONE
