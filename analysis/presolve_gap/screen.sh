#!/bin/bash
# 55 問で前処理後の大きさを設定ごとに比べる (1 秒で打ち切り、前処理の行だけ見る)。使い方: screen.sh <出力> <設定,...>
D=$(cd $(dirname "$0") && pwd); . $D/cfg.sh
out=$1; IFS=, read -ra CS <<< "$2"
for mps in /home/user/enomoto_solver/.miplib_cache/*.mps; do
  p=$(basename $mps .mps); line="$p"
  for c in "${CS[@]}"; do
    s=$(env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 $(cfg $c) timeout 300 python3 $D/run_eno.py $mps 1 2>&1 | grep -E "^Presolve:|^CHECK" | tr '\n' ' ' | sed 's/Presolve: //; s/ rows, / x /; s/ columns -> / -> /; s/ rows, / x /; s/ columns / /')
    line="$line | $c: $s"
  done
  echo "$line" >> $D/logs/$out
done
echo DONE >> $D/logs/$out
