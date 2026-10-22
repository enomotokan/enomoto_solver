#!/bin/bash
# 4 スレッド・60 秒: batch60.sh <ラベル> [環境変数...]  (13 問を 1 回ずつ。回数は呼ぶ側で)
D=$(cd $(dirname $0) && pwd); L=$1; shift
for P in roll3000 mzzv42z neos-911970 physiciansched6-2 beasleyC3 mc11 misc07 mik-250-20-75-4 qnet1 10teams neos5 h80x6320d p0201; do
  $D/run_eno.sh $P b_${P}_$L 60 4 "$@"
done
