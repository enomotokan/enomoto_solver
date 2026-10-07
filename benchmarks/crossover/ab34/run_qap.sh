#!/bin/bash
# qap15 の仕上げの進み具合 (同じ時間で何反復進むか) を、LU の稠密切替の案ごとに比べる (各 600 秒、ENOMOTO_DEBUG_PROGRESS)。
# 内点法 + クロスオーバーと二段解法の両方。第 33 回の後に走らせる。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab33/done.txt ]; do sleep 30; done
. .venv/bin/activate && maturin develop --release > benchmarks/crossover/ab34/build.log 2>&1
A="ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=2000 ENOMOTO_T_LU_DENSE_SWITCH_AUTO_LU_PER_ROW=16 ENOMOTO_T_LU_DENSE_SWITCH_AUTO=0.1"
C="ENOMOTO_LU_DENSE_SWITCH=0.1"
for solver in ipm_crossover simplex; do
  for v in "base X=0" "A $A" "C $C"; do
    set -- $v; name=$1; shift
    env "$@" ENOMOTO_DEBUG_PROGRESS=1 ENOMOTO_DEBUG_CROSSOVER=1 timeout 600 .venv/bin/python /tmp/claude-0/-home-user-enomoto-solver/fde3ce0a-29a7-542c-bada-627585e735fc/scratchpad/check.py /tmp/claude-0/-home-user-enomoto-solver/fde3ce0a-29a7-542c-bada-627585e735fc/scratchpad/qap/problem.mps $solver /tmp/claude-0/-home-user-enomoto-solver/fde3ce0a-29a7-542c-bada-627585e735fc/scratchpad/x_tmp.npy > benchmarks/crossover/ab34/qap_${solver}_${name}.log 2>&1
  done
done
echo ALL_DONE >> benchmarks/crossover/ab34/done.txt
