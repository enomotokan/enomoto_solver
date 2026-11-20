#!/bin/sh
# 使い方: go.sh 設定名 問題...  (設定名 def / lmc)
cd /home/user/enomoto_solver
O=$(dirname "$0")/logs; mkdir -p $O
c=$1; shift
for p in "$@"; do
  m=$(python -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  if [ "$c" = lmc ]; then export ENOMOTO_MIP_LOCALMIP=1 ENOMOTO_MIP_LOCALMIP_CHAIN=1; else unset ENOMOTO_MIP_LOCALMIP ENOMOTO_MIP_LOCALMIP_CHAIN; fi
  ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 python scripts/run_miplib_benchmark.py --worker-ours $m --time-limit 60 --threads 4 > $O/$p.$c.out 2> $O/$p.$c.log
  echo "$p $c $(tail -1 $O/$p.$c.out | cut -c1-200)"
done
