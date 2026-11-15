#!/bin/bash
# 重い 5 問を既定 (diag) と試作で 4 スレッド・300 秒ずつ回す。使い方: run_heavy.sh 出力.jsonl rep 設定...
D=$(cd $(dirname $0); pwd)
OUT=$1; REP=$2; shift 2
for p in supportcase33 ns1830653 tbfp-network rococoC10-001000 traininstance6; do
  for c in "$@"; do python3 $D/run1.py $D/$OUT $c $p 300 $REP; done
done
