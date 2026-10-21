#!/bin/bash
# 使い方: mkdebug.sh <問題> [HiGHS の秒]: 前処理後 (クリーク行の拡張の後) の問題を書き出し、HiGHS で解いてデバッグ解 <問題>_pre.sol を作る
cd "$(dirname "$0")"; p=$1; t=${2:-600}
ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=$PWD/${p}_pre.mps python run1.py ../../.miplib_cache/$p.mps 1 1 > /dev/null 2>&1
nice -n 19 python hsol2.py ${p}_pre.mps ${p}_pre.sol $t
