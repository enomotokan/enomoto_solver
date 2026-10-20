#!/bin/bash
# 副作用の比較: side.sh <out.tsv> <reps> <T> <problems...>。設定は CFGS (名前:環境変数 ...、;区切り)
out=$1; reps=$2; T=$3; shift 3
IFS=';' read -ra cfgs <<< "$CFGS"
for r in $(seq 1 $reps); do for p in "$@"; do for c in "${cfgs[@]}"; do
  name=${c%%:*}; envs=${c#*:}; [ "$envs" = "$c" ] && envs=""
  f=$(ls /home/user/enomoto_solver/.miplib_cache/$p.mps* | head -1)
  res=$(env ENOMOTO_MIP_THREADS=4 PYTHONPATH=/home/user/proto/.pylib $envs python /home/user/enomoto_solver/analysis/drayage_runs/eno_run.py $f $T 2>/dev/null | grep '^RESULT')
  echo -e "$r\t$p\t$name\t$res" >> $out
done; done; done
