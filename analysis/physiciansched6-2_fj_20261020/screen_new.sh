#!/bin/bash
# screen_new.sh <秒>: 40 問それぞれで、新しい試作が何をするかを数える
# (双対の上限の数、目的の列の代入で前処理後の大きさが変わるか、FJ の LP の修復で解が出た回数、DOMCOL の予測の境界の固定、DUALPROP の固定)
t=$1
D=$(cd "$(dirname "$0")" && pwd); R=/home/user/enomoto_solver
P=$(python3 -c "import sys; sys.path.insert(0,'$R/scripts'); import run_miplib_benchmark as r; print(' '.join(r.PROBLEMS))")
cd /tmp
for p in $P; do
  o0=$(ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_THREADS=4 timeout $((t+60)) python -I $R/analysis/twovar_20261020/eno_run.py $R/.miplib_cache/$p.mps $t 2>&1)
  o1=$(ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DOMCOL_PRED=1 ENOMOTO_MIP_DUALPROP=1 ENOMOTO_DEBUG_DUALPROPAGATE=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_THREADS=4 timeout $((t+60)) python -I $R/analysis/twovar_20261020/eno_run.py $R/.miplib_cache/$p.mps $t 2>&1)
  o2=$(ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 timeout $((t+60)) python -I $R/analysis/twovar_20261020/eno_run.py $R/.miplib_cache/$p.mps $t 2>&1)
  ub=$(echo "$o1" | grep "continuous column bounds by dual" | awk '{s+=$4} END{print s+0}')
  pre0=$(echo "$o0" | grep "^Presolve:" | head -1 | sed 's/.*-> //; s/ (.*//')
  pre1=$(echo "$o1" | grep "^Presolve:" | head -1 | sed 's/.*-> //; s/ (.*//')
  fj0=$(echo "$o0" | grep -c "FJ2: .*found true")
  fj1=$(echo "$o1" | grep -c "FJ2.*found true")
  rep=$(echo "$o1" | grep -c "LP repair: found")
  dc1=$(echo "$o1" | grep "dominated binaries fixed" | awk '{s+=$3} END{print s+0}')
  dc2=$(echo "$o2" | grep "dominated binaries fixed" | awk '{s+=$3} END{print s+0}')
  dp=$(echo "$o1" | grep "DEBUG_MIP_DUALPROP" | sed 's/.*fixed_columns=\([0-9]*\) (integer \([0-9]*\)).*/\1 \2/' | awk '{s+=$1; t+=$2} END{print s+0 "/" t+0}')
  echo "$p dualub=$ub pre=[$pre0]->[$pre1] fj_found=$fj0->$fj1 repair=$rep domcol=$dc2->$dc1(pred) dualprop(all/int)=$dp"
done
