#!/bin/sh
# 他の問題での確かめ (ノードの LP の数字つき): CFGS="名前:環境変数=値 ..." sh side2.sh <出力 jsonl> <秒> <スレッド> <問題 ...>
# 名前 def は既定。環境変数が複数なら , でつなぐ。種 1・ENOMOTO_MIP_LOG=1 で回し、結果の行と「node diag」(主のスレッド) の数字を 1 行の JSON にする。リポジトリの根で回す
out=$1; tl=$2; th=$3; shift 3
S=$(mktemp)
for p in "$@"; do
  for c in ${CFGS:-def}; do
    name=${c%%:*}; e=$(echo "${c#*:}" | tr , " "); [ "$name" = def ] && e="X=1"
    r=$(env $e ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th 2>$S | grep '^{' | tail -1)
    d=$(grep "MIP: node diag: nodes" $S | tail -1 | sed -E 's/.*node LPs ([0-9]+) mean rows ([0-9.]+) iters ([0-9]+) \(([0-9.]+)\/LP\) LP secs ([0-9.]+).*/{"node_lps": \1, "mean_rows": \2, "iters": \3, "iters_per_lp": \4, "lp_secs": \5}/')
    [ -z "$d" ] && d=null
    echo "{\"problem\": \"$p\", \"cfg\": \"$name\", \"threads\": $th, \"result\": $r, \"diag\": $d}" | tee -a $out
  done
done
rm -f $S
