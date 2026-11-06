#!/bin/bash
# 使い方: run1.sh <回の名前> <問題> [環境変数 ...]  (ENOMOTO、4 スレッド・600 秒、ログは logs/<回>_<問題>.{out,err})
cd /home/user/enomoto_solver
tag=$1; p=$2; shift 2
TL=${TL:-600}
o=analysis/submip_impr_20261106/B/logs/${tag}_${p}
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $TL --threads 4 > $o.out 2> $o.err
echo "$tag $p $(tail -1 $o.out | cut -c1-200)"
