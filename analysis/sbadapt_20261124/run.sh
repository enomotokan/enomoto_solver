#!/bin/bash
# 1 回解いて結果とログ (特徴の診断 SBADAPT_FEAT を含む) を残す。リポジトリの根から:
#   bash analysis/sbadapt_20261124/run.sh <名札> <問題> <秒> [K=V ...]
# satellites2 は ENOMOTO_MIP_HO4_ACFIX=d を自動で付ける。結果は res.jsonl、ログは logs/<名札>_<問題>.log.gz
cd "$(dirname "$0")/../.."
D=analysis/sbadapt_20261124; tag=$1; p=$2; tl=$3; shift 3
extra=""; case $p in satellites2*) extra="ENOMOTO_MIP_HO4_ACFIX=d";; esac
f=$D/logs/${tag}_$p.log
env ENOMOTO_MIP_SBADAPT_DIAG=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 $extra "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 > $f 2>&1
r=$(grep '^{' $f | tail -1); gzip -f $f
echo "{\"tag\": \"$tag\", \"problem\": \"$p\", \"tl\": $tl, \"env\": \"$extra $*\", \"result\": ${r:-null}}" >> $D/res.jsonl
echo "$tag $p $(echo $r | cut -c1-150)"
