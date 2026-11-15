#!/bin/bash
# MIPLIB で非有界 (solu の unbd) の問題を、直しの入ったビルド (PYTHONPATH) で 1 スレッド・120 秒で解く。
# 使い方: unbd.sh <設定名> [K=V ...]  → unbd_<設定名>.jsonl
cd $(dirname $0); n=$1; shift
for p in neos-3709489-menik neos-3699044-maipo neos-3672928-linge neos-3654993-kolva; do
  mps=$(python -c "import sys; sys.path.insert(0,'/home/user/wt_unb/scripts'); import run_miplib_benchmark as rb; from pathlib import Path; print(rb.fetch('$p', Path('/home/user/enomoto_solver/.miplib_cache')))")
  r=$(env "$@" ENOMOTO_MIP_LOG=1 nice python /home/user/wt_unb/scripts/run_miplib_benchmark.py --worker-ours $mps --time-limit 120 --threads 1 2> unbd_${n}_$p.log | grep '^{')
  echo "{\"problem\": \"$p\", \"cfg\": \"$n\", \"result\": $r}" >> unbd_$n.jsonl
  gzip -f unbd_${n}_$p.log
done
