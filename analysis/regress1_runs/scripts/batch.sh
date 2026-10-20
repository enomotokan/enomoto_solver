#!/bin/bash
# batch.sh <tag> <name> <T> <reps> <problems...> ; 追加の環境変数はそのまま渡る
tag=$1; name=$2; T=$3; reps=$4; shift 4
for r in $(seq 1 $reps); do for p in "$@"; do
  /home/user/wt/scripts/run.sh $name $p $T /home/user/wt/logs/${tag}_${p}_r$r.log
done; done
