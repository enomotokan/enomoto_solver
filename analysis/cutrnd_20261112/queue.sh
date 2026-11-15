#!/bin/bash
# 使い方: bash queue.sh 秒 種 設定,設定 問題 問題 ...  (ビルド a、同じ機械でソルバーは同時に 1 つ)
cd "$(git rev-parse --show-toplevel)"
TL=$1; S=$2; CF=$3; shift 3
for p in "$@"; do for c in ${CF//,/ }; do python analysis/cutrnd_20261112/run.py a $c $S $TL $p; done; done
