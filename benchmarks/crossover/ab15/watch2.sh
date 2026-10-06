#!/bin/bash
# 第 15 回の続きの計測が落ちたら再開する見張り。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab15/done2.txt ]; do
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab15/run_ab2\.sh"; then
    (setsid nohup benchmarks/crossover/ab15/run_ab2.sh > /dev/null 2>&1 < /dev/null &)
  fi
  sleep 60
done
