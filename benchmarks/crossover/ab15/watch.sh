#!/bin/bash
# 第 15 回の計測が落ちたら再開する見張り。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab15/done.txt ]; do
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab15/run_ab\.sh"; then
    (setsid nohup benchmarks/crossover/ab15/run_ab.sh > /dev/null 2>&1 < /dev/null &)
  fi
  sleep 60
done
