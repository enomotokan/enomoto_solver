#!/bin/bash
# 注意: この計測のときの ENOMOTO_MIP_RESTORE_READD=2 は今の =3 (手元と共有の両方)
# 4 スレッドの比較 (各 2 回)
cd /home/user/enomoto_solver
for r in 1 2; do
for v in "def X=1" "A2 ENOMOTO_MIP_RESTORE_READD=2" "D ENOMOTO_MIP_RESTORE_NB=1" "A2D ENOMOTO_MIP_RESTORE_READD=2 ENOMOTO_MIP_RESTORE_NB=1"; do
  set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_r$r mc11 60 "$@"
done
done
for r in 1 2; do
for v in "def X=1" "D ENOMOTO_MIP_RESTORE_NB=1" "A2D ENOMOTO_MIP_RESTORE_READD=2 ENOMOTO_MIP_RESTORE_NB=1" "A2 ENOMOTO_MIP_RESTORE_READD=2"; do
  set -- $v; tag=$1; shift
  runs_nr/ab.sh ${tag}_co_r$r neos-2978193-inde 120 ENOMOTO_MIP_CUTOFF=-2.38806169 "$@"
done
done
