#!/bin/sh
# OBJTARGET 9 (下からの T) と 8 (出発点の比べ)。
cd "$(dirname "$0")"
P9="neos-933966 sing326 trento1 brazil3 momentum1 neos-3024952-loue neos-5107597-kakapo gen-ip002 assign1-5-8"
for s in 1 2; do
  python traj.py D_def 60 $s $P9
  ENOMOTO_MIP_TPRE=b python traj.py D_mid 60 $s $P9
  ENOMOTO_MIP_TPRE=db python traj.py D_lad 60 $s $P9
done
# 8: 出発点の比べ (前処理の後で止める)。解なしの問題と解が悪い問題
for p in academictimetablesmall satellites2-60-fs satellites2-40 rail01 neos-3216931-puriri dws008-01 cmflsp50-24-8-8 ns1952667 neos-1354092 bnatt400 neos-2746589-doon germanrr peg-solitaire-a3 neos-3024952-loue neos-5107597-kakapo comp21-2idx; do
  mps=$(python -c "import sys;sys.path.insert(0,'../../scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('../../.miplib_cache').resolve()))")
  echo "== $p"
  (cd ../.. && ENOMOTO_MIP_TPRE=s ENOMOTO_MIP_DEBUG_PRETIME=stop ENOMOTO_MIP_THREADS=4 timeout 900 python scripts/run_miplib_benchmark.py --worker-ours "$mps" --time-limit 600 --threads 4 2>&1 | grep -E "TPRE_START|PRETIME total")
done > start8.txt
