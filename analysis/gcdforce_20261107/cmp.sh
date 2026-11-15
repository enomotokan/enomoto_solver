#!/bin/bash
# 使い方: cmp.sh 問題 秒 出力.jsonl  (既定・GCDFORCE・GCDFORCE+HNFPRE=bounds を種 1・2 で)
p=$1; T=$2; out=$3
cd "$(dirname "$0")/../.."
for s in 1 2; do
  for v in def gcd gcdhnf; do
    case $v in def) e="";; gcd) e="ENOMOTO_MIP_GCDFORCE=1";; gcdhnf) e="ENOMOTO_MIP_GCDFORCE=1 ENOMOTO_MIP_HNFPRE=bounds";; esac
    r=$(env $e ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $T --threads 4 2>/tmp/claude-0/cmp_err.txt | tail -1)
    echo "{\"problem\": \"$p\", \"variant\": \"$v\", \"seed\": $s, \"result\": $r}" | tee -a $out
  done
done
