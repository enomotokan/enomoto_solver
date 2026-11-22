#!/bin/bash
# traininstance6 の not_solved の再現: def (環境変数なし)、4 スレッド、600 秒、ENOMOTO_MIP_LOG=1。使い方: ./rep.sh SEED TAG [VAR=値 ...] (足す環境変数)
# ログは logs/ti6_sSEED_TAG.log.gz、結果の行は rep.jsonl に足す。
cd "$(dirname "$0")/../.."
S=$1; T=$2; shift 2
L=analysis/notsolved_20261122/logs/ti6_s${S}_${T}.log
env $(env | grep -o '^ENOMOTO_[^=]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$S ENOMOTO_MIP_LOG=1 "$@" \
  python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/traininstance6.mps --time-limit 600 --threads 4 >"$L.out" 2>"$L"
R=$(grep '^{' "$L.out" | tail -1)
echo "{\"seed\": $S, \"tag\": \"$T\", \"at\": \"$(date +%H:%M:%S)\", \"result\": ${R:-null}}" >> analysis/notsolved_20261122/rep.jsonl
gzip -f "$L"; rm -f "$L.out"
echo "$R"
