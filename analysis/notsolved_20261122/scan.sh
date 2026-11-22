#!/bin/bash
# 葉の結論なし (整数列がすべて固定された葉で LP が解けない) が出るかの走査: 4 スレッド、ENOMOTO_MIP_LOG=1。
# 使い方: ./scan.sh PROB TL TAG SEED... [-- VAR=値 ...]。ログは捨て、要約の行 (状態・葉の数・結論なし) を scan.jsonl に足す。
cd "$(dirname "$0")/../.."
P=$1; TL=$2; T=$3; shift 3
SEEDS=(); while [ $# -gt 0 ] && [ "$1" != "--" ]; do SEEDS+=("$1"); shift; done; [ "${1:-}" = "--" ] && shift
for S in "${SEEDS[@]}"; do
  L=$(mktemp)
  env $(env | grep -o '^ENOMOTO_[^=]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$S ENOMOTO_MIP_LOG=1 "$@" \
    python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $TL --threads 4 >"$L.out" 2>"$L"
  R=$(grep '^{' "$L.out" | tail -1)
  LEAF=$(grep -a "leaves with all integer columns fixed" "$L" | tail -1 | sed 's/^MIP: //' | sed 's/"/\x27/g')
  LPINF=$(grep -a "marginal LP infeasibility" "$L" | tail -1 | sed 's/^MIP: //')
  echo "{\"problem\": \"$P\", \"seed\": $S, \"tag\": \"$T\", \"result\": ${R:-null}, \"leaf\": \"$LEAF\", \"lpinf\": \"$LPINF\"}" >> analysis/notsolved_20261122/scan.jsonl
  rm -f "$L" "$L.out"
done
