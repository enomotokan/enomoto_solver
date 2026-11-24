#!/bin/bash
# 追加の害の候補 5 問 (1 スレッド、種 1、600 秒): 既定・combo・combo+k+s (NODEROWS_ADAPT=tk・ENUM_ADAPT=1s)。
# bash analysis/comboharm_20261124/more.sh <python> 並べる本数
cd "$(dirname "$0")/../.."
PY=$1; PAR=$2; R=analysis/comboharm_20261124
jobs_list() { for p in rail507 neos-2987310-joes tbfp-network gen-ip054 roi2alpha3n4; do echo "$p def,combo -"; echo "$p combo ks"; done; }
jobs_list | xargs -P $PAR -L 1 sh -c '
  if [ "$2" = ks ]; then CH_EXTRA="ENOMOTO_MIP_NODEROWS_ADAPT=tk ENOMOTO_MIP_ENUM_ADAPT=1s" '"$PY"' '"$R"'/run.py '"$R"'/res_more_ks.jsonl $0 1 $1 --par 1;
  else '"$PY"' '"$R"'/run.py '"$R"'/res_more.jsonl $0 1 $1 --par 2; fi'
