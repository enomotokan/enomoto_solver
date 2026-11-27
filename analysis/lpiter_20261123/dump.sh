#!/bin/bash
# 木のノードの LP を書き出す (ENOMOTO_MIP_NODELP_DUMP、解き方は変えない)。リポジトリの根から:
#   bash analysis/lpiter_20261123/dump.sh <問題> <秒> <何回に 1 回> [K=V ...]
# 書き出し先: analysis/lpiter_20261123/dumps/<問題>/ (コミットしない)、ログ: logs/<問題>_dump.log.gz
cd "$(dirname "$0")/../.."
D=analysis/lpiter_20261123; p=$1; tl=$2; every=$3; shift 3
mkdir -p $D/dumps/$p $D/logs
env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_NODELP_DIAG=1 \
  ENOMOTO_MIP_NODELP_DUMP=$D/dumps/$p ENOMOTO_T_MIP_NODELP_DUMP_EVERY=$every ENOMOTO_T_MIP_NODELP_DUMP_MAX=30 \
  python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 2>&1 | gzip > $D/logs/${p}_dump.log.gz
zcat $D/logs/${p}_dump.log.gz | grep '^{' | tail -1 | cut -c1-200
wc -l < $D/dumps/$p/index.txt
