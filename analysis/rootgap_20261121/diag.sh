#!/bin/sh
# 遅れの調べ (nw04・qiu): 設定 def・glc・go (ENOMOTO_MIP_GEN_OLD=1 だけ)・lc (ENOMOTO_MIP_LIFTED_COVER=1 だけ) で、ログ (ENOMOTO_MIP_LOG=1・SEPSTAT=1) を
# logs/diag_<問題>_<設定>_t<スレッド>_s<種>.log に残す。使い方: sh diag.sh <スレッド> "<種…>" [問題…]  (60 秒)
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
TH=$1; SEEDS=$2; shift 2
PS=${*:-"nw04 qiu"}
for p in $PS; do for s in $SEEDS; do for c in def glc go lc; do
  case $c in glc) E="ENOMOTO_MIP_GEN_OLD=1 ENOMOTO_MIP_LIFTED_COVER=1";; go) E="ENOMOTO_MIP_GEN_OLD=1";; lc) E="ENOMOTO_MIP_LIFTED_COVER=1";; *) E="";; esac
  f=logs/diag_${p}_${c}_t${TH}_s${s}.log
  [ -s "$f" ] && grep -q @@RESULT "$f" && continue
  env $E ENOMOTO_MIP_SEPSTAT=1 ENOMOTO_T_MIP_SEED=$s python "$R/analysis/smallprob_overhead_20261028/prof_run.py" ours "$R/.miplib_cache/$p.mps" "$f" --threads $TH --time-limit 60 >/dev/null 2>&1
  echo "$p $c t$TH s$s $(tail -1 $f)"
done; done; done
