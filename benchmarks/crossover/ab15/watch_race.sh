#!/bin/bash
# 同時実行の比較が落ちたら再開する見張り。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab15/done_race.txt ]; do
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab15/run_race\.sh"; then
    (setsid nohup benchmarks/crossover/ab15/run_race.sh > /dev/null 2>&1 < /dev/null &)
  fi
  sleep 60
done
