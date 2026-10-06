#!/bin/bash
# 第 12 回の計測が落ちたら再開する見張り (コンテナの再起動などで止まったとき)。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab12/done.txt ]; do
  sleep 60
  if ! ps -eo cmd | grep -qE "^/bin/bash benchmarks/crossover/ab12/(chain|run_ab)\.sh"; then
    (setsid nohup benchmarks/crossover/ab12/chain.sh > /dev/null 2>&1 < /dev/null &)
  fi
done
