#!/bin/bash
# 第 16 回の計測が落ちたら再開する見張り。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab16/done.txt ]; do
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab16/run_ab_regfloor\.sh"; then
    (setsid nohup benchmarks/crossover/ab16/run_ab_regfloor.sh > /dev/null 2>&1 < /dev/null &)
  fi
  sleep 60
done
