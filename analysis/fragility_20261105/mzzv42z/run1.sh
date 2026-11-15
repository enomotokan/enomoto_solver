#!/bin/bash
# 使い方: run1.sh ラベル 種 スレッド数 [追加の環境変数 ...]
# ログは analysis/fragility_20261105/mzzv42z/log_<ラベル>.txt、結果 (JSON) は .stdout
set -u
cd "$(dirname "$0")/../../.."
lab=$1; seed=$2; th=$3; shift 3
out=analysis/fragility_20261105/mzzv42z/log_${lab}
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$seed "$@" \
  timeout 1000 python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/mzzv42z.mps --time-limit 600 --threads $th \
  > ${out}.txt.stdout 2> ${out}.txt
echo "$lab $(cat ${out}.txt.stdout)"
