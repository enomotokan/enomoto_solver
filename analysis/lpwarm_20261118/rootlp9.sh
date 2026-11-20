#!/bin/bash
# 根の LP 9 問 (analysis/rootlp_simplex_fail_20261105 の組) を書き出し、cut_lp_debug (全スラック基底から) で既定と試作 gz を比べる。
# 使い方: bash rootlp9.sh <書き出し先> <テストの実行ファイル>   出力: 1 行 1 問 1 設定
D=$1; TB=$2; mkdir -p $D
WT=$(cd "$(dirname "$0")/../.." && pwd)
for p in supportcase42 uccase12 uccase9 snp-02-004-104 sing326 sing44 rail01 neos-873061 thor50dday; do
  mps=$($WT/.venv/bin/python -c "import sys; sys.path.insert(0,'$WT/scripts'); import run_miplib_benchmark as rb; from pathlib import Path; print(rb.fetch('$p', Path('$WT/.miplib_cache')))")
  [ -f $D/$p.mps ] || ENOMOTO_MIP_ROOTLP_SIMPLEX_DIAG=$D/$p ENOMOTO_MIP_ROOTLP_DUMP_ONLY=1 ENOMOTO_MIP_THREADS=1 timeout 900 $WT/.venv/bin/python $WT/scripts/run_miplib_benchmark.py --worker-ours $mps --time-limit 60 --threads 1 > /dev/null 2>&1
  for e in X=1 ENOMOTO_SIMPLEX_BFRT_GROUP=gz; do
    r=$(env CUTLP=$D/$p CUTLP_COLD=1 CUTLP_TIME=200 $e $TB cut_lp_debug --ignored --nocapture 2>&1 | grep -o "status=.*obj=[-0-9.e+]*")
    echo "$p $e $r"
  done
done
