#!/bin/bash
# rocI-4-11 の glc の誤答の再現: rep.sh <札> <スレッド> <種...> -- K=V ...  (デバッグ解 roci_opt.sol と ENOMOTO_MIP_LOG=1。ログ logs/、結果 rep.jsonl)
cd "$(dirname "$0")/../../.."
D=analysis/rootgap_20261121/wrong_roci; mkdir -p $D/logs
tag=$1; th=$2; shift 2; S=(); while [ $# -gt 0 ] && [ "$1" != "--" ]; do S+=("$1"); shift; done; [ "${1:-}" = "--" ] && shift; KV=("$@")
for s in "${S[@]}"; do
  f=$D/logs/${tag}_t${th}_s${s}.log
  env "${KV[@]}" ENOMOTO_MIP_DEBUG_SOL=$PWD/$D/roci_opt.sol ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s timeout 900 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/rocI-4-11.mps --time-limit 600 --threads $th > $f 2>&1
  out=$(grep '^{' $f | tail -1); nd=$(grep -c "MIP_DEBUG_SOL" $f); lost=$(grep -m1 "MIP_DEBUG_SOL lost\|violated by the debug\|cuts off the debug" $f | cut -c1-200 | sed 's/"/\x27/g')
  gzip -f $f
  echo "{\"tag\": \"$tag\", \"threads\": $th, \"seed\": $s, \"env\": \"${KV[*]}\", \"dbg_lines\": $nd, \"first_dbg\": \"$lost\", \"result\": ${out:-null}}" >> $D/rep.jsonl
  echo "$tag t$th s$s $(echo $out | cut -c1-90) dbg=$nd"
done
