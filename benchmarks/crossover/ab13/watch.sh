#!/bin/bash
# 第 13 回の計測が落ちたら再開する見張り。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab13/done.txt ]; do
  sleep 60
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab13/run_ab\.sh"; then
    (setsid nohup benchmarks/crossover/ab13/run_ab.sh > /dev/null 2>&1 < /dev/null &)
  fi
done
