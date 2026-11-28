#!/bin/bash
# PRESIZE240 (presolve2_20261111 の写しに、MPS の取得と後始末を足した): HiGHS の前処理を、規則を 1 つずつ切って回す
# (presolve_rule_off のビット 6〜19)。出力は rules.jsonl に追記。キャッシュになかった MPS は回した後に消す。
cd "$(dirname "$0")"
C=../../.miplib_cache
for p in "$@"; do
  had=1; [ -s $C/$p.mps ] || had=0
  [ $had = 1 ] || python3 -c "import sys;sys.path.insert(0,'../../scripts');import run_miplib_benchmark as rb;from pathlib import Path;rb.fetch('$p',Path('$C'))"
  for off in 0 64 128 256 512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288; do
    timeout 600 python3 highs_rules.py $p $off >> rules.jsonl || echo "{\"problem\":\"$p\",\"off\":$off,\"fail\":1}" >> rules.jsonl
  done
  [ $had = 1 ] || rm -f $C/$p.mps
done
