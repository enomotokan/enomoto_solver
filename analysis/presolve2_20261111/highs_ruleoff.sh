#!/bin/bash
# PRESOLVE2: HiGHS の前処理を、規則を 1 つずつ切って回す (presolve_rule_off のビット 6〜19)。出力は rules.jsonl に追記。
cd "$(dirname "$0")"
for p in "$@"; do
  for off in 0 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288; do
    timeout 600 python3 highs_rules.py $p $off >> rules.jsonl || echo "{\"problem\":\"$p\",\"off\":$off,\"fail\":1}" >> rules.jsonl
  done
done
