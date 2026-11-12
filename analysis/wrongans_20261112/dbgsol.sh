#!/bin/bash
# 既知解つきの診断 (analysis/wrongans_20261105 §2 と同じ): G5 で書き出した traininstance6 の縮約後の問題 (pre_t6_prr.mps) を
# 4 スレッド・400 秒で解き、HiGHS の最適解 28290 を ENOMOTO_MIP_DEBUG_SOL に渡す。既知解を切る衝突・ノードを数える。
# 使い方: dbgsol.sh <名前> [K=V ...]   (例: dbgsol.sh def、dbgsol.sh old ENOMOTO_MIP_CONFLICT_INT_TOL=0)
cd $(dirname $0); S=/tmp/claude-0/-home-user-enomoto-solver/5ffdcc17-b65f-5493-9321-58806e0e3ef7/scratchpad
n=$1; shift
env "$@" ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_SOL=$S/sol.txt \
  python ../../scripts/run_miplib_benchmark.py --worker-ours $S/pre_t6_prr.mps --time-limit 400 --threads 4 > dbg_$n.json 2> dbg_$n.log
gzip -f dbg_$n.log
