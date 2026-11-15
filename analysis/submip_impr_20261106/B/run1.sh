#!/bin/bash
# 使い方: run1.sh <回の名前> <問題> [環境変数 ...]  (ENOMOTO、4 スレッド・600 秒、ログは logs/<回>_<問題>.{out,err})
# 書き込み中のログはリポジトリの外 (/tmp/claude-0/running) に置き、終わってから logs/ に移す
cd /home/user/enomoto_solver
tag=$1; p=$2; shift 2
TL=${TL:-600}
o=/tmp/claude-0/running/${tag}_${p}
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $TL --threads 4 > $o.out 2> $o.err
mv $o.out $o.err analysis/submip_impr_20261106/B/logs/
echo "$tag $p $(tail -1 analysis/submip_impr_20261106/B/logs/${tag}_${p}.out | cut -c1-200)"
