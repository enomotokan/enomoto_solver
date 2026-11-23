#!/bin/sh
# 他の問題での確かめ: CFGS="名前=ENOMOTO_MIP_NODE_CLEANUP の値 ..." sh side.sh <出力 jsonl> <秒> <問題 ...>
# (1 スレッド・種 1。名前 def は既定。CFGS を省くと "def qs=qs")。リポジトリの根で回す
out=$1; tl=$2; shift 2
for p in "$@"; do
  for c in ${CFGS:-def qs=qs}; do
    name=${c%%=*}; v=${c#*=}
    if [ "$name" = def ]; then e="X=1"; else e="ENOMOTO_MIP_NODE_CLEANUP=$v"; fi
    r=$(env $e ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 2>/dev/null | grep '^{' | tail -1)
    echo "{\"problem\": \"$p\", \"cfg\": \"$name\", \"result\": $r}" | tee -a $out
  done
done
