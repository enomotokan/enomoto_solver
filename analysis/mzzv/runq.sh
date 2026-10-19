#!/bin/bash
# 1 行 1 本: <ログ名> <問題> <秒> [環境変数 ...] を順に (ソルバーは同時に 1 つ)。ログは t4/<ログ名>.log
cd /home/user/enomoto_solver/analysis/mzzv
export ENOMOTO_MIP_THREADS=4
while read -r name prob secs envs; do
  [ -z "$name" ] && continue
  case "$name" in \#*) continue;; esac
  if [[ "$name" == highs* ]]; then
    env $envs timeout $((${secs%.*}*2+120)) python run_highs.py $prob $secs > t4/$name.log 2>&1
  else
    env ENOMOTO_MIP_LOG=1 $envs timeout $((${secs%.*}*2+120)) python run_eno.py $prob $secs > t4/$name.log 2>&1
  fi
  echo "$name: $(grep RESULT t4/$name.log | cut -c1-300)"
done
