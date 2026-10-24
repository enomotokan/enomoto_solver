#!/bin/bash
# HiGHS の前処理の規則を 1 つずつ切って前処理後の問題を作り、ENOMOTO (既定、12 秒) の前処理後・根の LP・1 ラウンド目を見る
cd "$(dirname "$0")"
for r in 0 256 4096 8192 16384 32768 65536 131072; do
  python3 highs_presolve_dump.py ../../.miplib_cache/physiciansched6-2.mps /tmp/claude-0/hpre_off$r.mps presolve_rule_off=$r 2>&1 | grep PRESOLVED | sed "s/^/off=$r /"
  ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 python3 eno_run.py /tmp/claude-0/hpre_off$r.mps 14 4 > logs/rule_off${r}.log 2>&1
  grep -E "^Presolve:|clique extension|MIP: root LP |cut round 0|Root cuts" logs/rule_off${r}.log | head -5 | sed "s/^/   /"
done
