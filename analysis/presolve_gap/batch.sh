#!/bin/bash
# 使い方: batch.sh <出力> <回数> <問題,...> <設定,...>  設定を交互に回す
D=$(cd $(dirname "$0") && pwd); . $D/cfg.sh
out=$1; n=$2; IFS=, read -ra PS <<< "$3"; IFS=, read -ra CS <<< "$4"
for r in $(seq 1 $n); do for p in "${PS[@]}"; do for c in "${CS[@]}"; do
  P=$p $D/erun.sh ${c}_$r $(cfg $c) >> $D/logs/$out
done; done; done
echo DONE >> $D/logs/$out
