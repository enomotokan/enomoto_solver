#!/bin/bash
# ENOMOTO の前処理後 (E2 = 同値の代入あり、クリーク拡張の前) に HiGHS の前処理をかけ (規則を 1 つずつ切る)、ENOMOTO (既定、14 秒) の根の LP を見る
# rule_sweep2.sh <ENOMOTO の dump の mps> <名前>
cd "$(dirname "$0")"
src=$1; tag=$2
for r in 0 4096 8192 32768 65536; do
  python3 highs_presolve_dump.py $src /tmp/claude-0/${tag}_h$r.mps presolve_rule_off=$r 2>&1 | grep PRESOLVED | sed "s/^/off=$r /"
  ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 python3 eno_run.py /tmp/claude-0/${tag}_h$r.mps 14 4 > logs/${tag}_h$r.log 2>&1
  grep -E "^Presolve:|MIP: root LP |cut round 0: [0-9]+ cuts \(|Root cuts" logs/${tag}_h$r.log | head -4 | sed "s/^/   /"
done
