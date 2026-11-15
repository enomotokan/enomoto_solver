#!/bin/bash
# 計装した HiGHS 1.15.1 (highs_nodelp.patch、HIGHS_NODELP で木のノードの LP 1 回ごとに `NLP` の行) で 1 問を解く。
# 使い方: run_highs.sh 問題 秒   (threads=4・mip_rel_gap=0。ログ logs/<問題>_highs.log、NLP の行 logs/<問題>_highs.nlp.gz)
set -e
D=$(cd $(dirname $0); pwd)
P=$1; T=${2:-300}
HB=${HIGHS_BIN:-/tmp/hs/HiGHS/build/bin/highs}
OPT=$(mktemp)
printf "threads = 4\nmip_rel_gap = 0\ntime_limit = %s\n" "$T" > $OPT
HIGHS_NODELP=1 $HB --options_file $OPT $D/../../.miplib_cache/$P.mps > $D/logs/${P}_highs.log 2> >(grep '^NLP' | gzip > $D/logs/${P}_highs.nlp.gz)
rm -f $OPT
