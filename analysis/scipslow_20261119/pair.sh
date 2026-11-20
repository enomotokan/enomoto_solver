#!/bin/bash
# 1 スレッドで SCIP より遅い問題を、当方 (1 スレッド、ENOMOTO_MIP_LOG=1) と SCIP (ENOMOTO_BENCH_SCIP_LOG=1) で解き、ログを並べる。
# 使い方 (リポジトリの根から): bash analysis/scipslow_20261119/pair.sh <種> <秒> <問題...> [-- K=V ...]  (K=V は当方にだけ付ける環境変数)
# ログ: analysis/scipslow_20261119/logs/<問題>_<ours|scip><札>_s<種>.log.gz、結果: res_pair.jsonl。札は環境 TAG (既定は空)
cd "$(dirname "$0")/../.."
D=analysis/scipslow_20261119; s=$1; tl=$2; shift 2
P=(); while [ $# -gt 0 ] && [ "$1" != "--" ]; do P+=("$1"); shift; done; [ "${1:-}" = "--" ] && shift; KV=("$@")
for p in "${P[@]}"; do
  for who in ours scip; do
    [ $who = scip ] && [ -n "${TAG:-}" ] && continue
    f=$D/logs/${p}_${who}${TAG:-}_s${s}.log
    if [ $who = ours ]; then
      env "${KV[@]}" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 > $f 2>&1
    else
      env ENOMOTO_BENCH_SCIP_LOG=1 ENOMOTO_BENCH_SCIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-scip .miplib_cache/$p.mps --time-limit $tl --threads 1 > $f 2>&1
    fi
    out=$(grep '^{' $f | tail -1)
    gzip -f $f
    echo "{\"problem\": \"$p\", \"who\": \"$who${TAG:-}\", \"seed\": $s, \"env\": \"${KV[*]}\", \"result\": ${out:-null}}" >> $D/res_pair.jsonl
    echo "$p $who${TAG:-} $s $(echo $out | cut -c1-150)"
  done
done
