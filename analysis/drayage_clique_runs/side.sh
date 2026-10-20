#!/bin/bash
# 比較: side.sh <out.tsv> <reps> <T> <problems...>。設定は CFGS (名前:環境変数 ...、;区切り)。ログは logs/<名前>_<問題>_<回>.log
d=/home/user/enomoto_solver/analysis/drayage_clique_runs
out=$1; reps=$2; T=$3; shift 3
IFS=';' read -ra cfgs <<< "$CFGS"
for r in $(seq 1 $reps); do for p in "$@"; do for c in "${cfgs[@]}"; do
  name=${c%%:*}; envs=${c#*:}; [ "$envs" = "$c" ] && envs=""
  f=/home/user/enomoto_solver/.miplib_cache/$p.mps
  log=$d/logs/${name}_${p}_$r.log
  (cd /tmp && env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 $envs python $d/eno_run.py $f $T > $log 2>&1)
  res=$(grep '^RESULT' $log)
  echo -e "$r\t$p\t$name\t$res" >> $out
done; done; done
