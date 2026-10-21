#!/bin/bash
# ENOMOTO_MIP_FLAT_RESTART=1 で 60 秒解き、再スタートの有無と最初の根のカットのループの伸びを見る
cd "$(dirname "$0")"
while read p; do
  [ -z "$p" ] && continue
  PROB=$p ./run.sh scanfr60 60 ENOMOTO_MIP_FLAT_RESTART=1 > /dev/null
  log=logs/${p}_scanfr60.log
  echo "$p | $(grep -c "^Restart" $log) | flat $(grep -c "flat root before" $log) | $(grep -m1 'cut loop done' $log | grep -o 'obj .*') | $(grep -m1 -E 'background cuts: .*(cut rows|not used|no result)' $log | cut -c1-90) | $(grep RESULT $log)"
done < ${LIST:-problems56.txt}
