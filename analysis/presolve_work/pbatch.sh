#!/bin/bash
# 55 問の前処理の後の形を出す。使い方: pbatch.sh <ラベル> <秒> <スレッド> <出力 jsonl> (環境変数はそのまま渡る)。済んだ問題は飛ばす
D=$(cd $(dirname "$0") && pwd); W=${W:-$D/work}; mkdir -p $W
for m in $D/../../.miplib_cache/*.mps; do
  p=$(basename $m .mps)
  [ -f $D/$4 ] && grep -q "\"name\":\"$p\",\"label\":\"$1\"" $D/$4 && continue
  $D/pone.sh $m $W $1 $2 $3 >> $D/$4
done
echo ALL DONE $1
