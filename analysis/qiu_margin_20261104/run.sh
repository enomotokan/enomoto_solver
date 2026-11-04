#!/bin/bash
# 使い方: run.sh 名前 スレッド数 時間の上限 [環境変数=値 ...]  (出力は analysis/qiu_margin_20261104/raw/名前.{out,log})
name=$1; th=$2; tl=$3; shift 3
d=/home/user/enomoto_solver/analysis/qiu_margin_20261104/raw; mkdir -p $d
cd /home/user/enomoto_solver
env ENOMOTO_MIP_LOG=1 "$@" timeout $((tl+60)) python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/qiu.mps --time-limit $tl --threads $th > $d/$name.out 2> $d/$name.log
echo "rc $?" >> $d/$name.out
