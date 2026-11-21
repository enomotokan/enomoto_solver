#!/bin/sh
# nw04 の遅れの調べ: def と glc を 1 スレッド・種 1〜3・60 秒で、ログ (ENOMOTO_MIP_LOG=1・SEPSTAT=1) を logs/nw04_<設定>_s<種>.log に残す。
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
for s in 1 2 3; do
  for c in def glc; do
    if [ $c = glc ]; then E="ENOMOTO_MIP_GEN_OLD=1 ENOMOTO_MIP_LIFTED_COVER=1"; else E=""; fi
    env $E ENOMOTO_MIP_SEPSTAT=1 ENOMOTO_T_MIP_SEED=$s python "$R/analysis/smallprob_overhead_20261028/prof_run.py" ours "$R/.miplib_cache/nw04.mps" "logs/nw04_${c}_s${s}.log" --threads 1 --time-limit 60 >/dev/null 2>&1
    echo "$c s$s $(tail -1 logs/nw04_${c}_s${s}.log)"
  done
done
