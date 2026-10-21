#!/bin/bash
# compare_rounds.sh <dumps/runN> <ENOMOTO のログ> -> ラウンドごとに ENOMOTO (MIP の中) と HiGHS (同じ基底から双対単体法) の反復数
cd "$(dirname "$0")"
dir=$1; log=$2
for f in $(ls $dir/*.mps | sort -t_ -k2.2n); do
  p=${f%.mps}; b=$(basename $p)
  r=${b#cutlp_}; r=${r%%_*}
  eno=$(grep "MIP: cut round ${r#r}: LP .* after .* iters" $log | head -1 | sed 's/.*LP \([A-Za-z]*\) after \([0-9]*\) iters (\([0-9.]*\)s).*/\1 \2 \3s/')
  python3 solve_dump.py $p 120 warm dual > logs/highs_$(basename $dir)_$b.log 2>&1
  h=$(grep RESULT logs/highs_$(basename $dir)_$b.log | sed 's/.*status=//')
  echo "$b | ENOMOTO: $eno | HiGHS: $h"
done
