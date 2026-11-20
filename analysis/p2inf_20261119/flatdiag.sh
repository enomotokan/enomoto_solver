#!/bin/bash
# P2-INF: cfx2 の FLAT の判定の診断。fiball・mcsched を種 2・4 スレッド・600 秒で cfx2 と LIT_EXPLAIN + HiGHS 式 (FLAT なし) で回す (logs/<問題>_<設定>_s2.log)。
# リポジトリの根から: PYTHONPATH=<ビルド> bash analysis/p2inf_20261119/flatdiag.sh
cd "$(dirname "$0")/../.."
L=analysis/p2inf_20261119/logs
for p in fiball mcsched; do
  f=$(python3 -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('.miplib_cache')))")
  for c in cfx2 lxhc; do
    case $c in cfx2) E="ENOMOTO_MIP_LIT_EXPLAIN=1 ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1 ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT=0.1";; lxhc) E="ENOMOTO_MIP_LIT_EXPLAIN=1 ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1";; esac
    env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=2 ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit 600 --threads 4 > $L/${p}_${c}_s2.log 2>&1
  done
done
