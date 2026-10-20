#!/bin/bash
# 使い方: runs/rep.sh <問題> <回数> <時間上限> <ログの接頭辞> [ENV=...]...
# ENOMOTO を 4 スレッドで <回数> 回動かし、暫定解の推移と終わりを 1 行ずつまとめる
p=$1; n=$2; T=$3; pre=$4; shift 4
cd "$(dirname "$0")/.."
export ENOMOTO_MIP_LOG=1
for i in $(seq 1 $n); do
  env ENOMOTO_MIP_THREADS=4 "$@" python runs/run_eno.py .miplib_cache/$p.mps $T 4 > $pre.$i.log 2>&1
  inc=$(grep -o "new incumbent [-0-9.e]* at node [0-9]* ([0-9.]*s) \[[^]]*\]" $pre.$i.log | sed 's/new incumbent //;s/at node //' | awk '{printf "%s@%s%s ", $1+0, $3, $4}')
  last=$(grep -E "^ +[0-9.]+s +[0-9]+" $pre.$i.log | tail -1 | awk '{print $1, "nodes", $2, "lb", $4, "ub", $5}')
  res=$(grep -o "RESULT [0-9.]* {'status': '[a-z_]*', 'objective': [-0-9.e]*" $pre.$i.log | sed "s/{'status': //;s/'objective': //")
  echo "$i | $res | $last | $inc"
done
