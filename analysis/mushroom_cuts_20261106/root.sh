#!/bin/bash
# 再スタートの後の模型 (/tmp/mdump/m_r1.mps) の根だけ (1 スレッド・ノード上限 1・打ち切り値 = 最適値) を 1 回。使い方: root.sh 名前 [KEY=VAL ...]
D=$(cd $(dirname $0); pwd); WT=$(cd $D/../..; pwd)
name=$1; shift
cd $WT && env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_NODE_LIMIT=1 ENOMOTO_MIP_CUTOFF=0.0553338 ENOMOTO_MIP_DEBUG_SEP=1 "$@" \
  python scripts/run_miplib_benchmark.py --worker-ours /tmp/mdump/m_r1.mps --time-limit 120 --threads 1 > $D/root/root_$name.out 2> $D/root/root_$name.err
gzip -f $D/root/root_$name.err
