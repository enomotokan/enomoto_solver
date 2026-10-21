#!/bin/bash
# 各ログの 再スタートの時刻 + 再スタート後に木に入る時刻 = 木に入る実時間、60 秒のノード数
cd "$(dirname "$0")"
for f in "$@"; do
  r=$(grep -m1 '^Restart 1' $f | grep -o '[0-9.]*s)' | tr -d 's)'); t=$(grep -m1 'tree search with' $f | grep -o '[0-9.]*s)' | tr -d 's)')
  echo "$(basename $f .log) restart=${r} tree=$(python3 -c "print(round(${r:-0}+${t:-0},1))") $(grep RESULT $f | grep -o 't=[0-9.]* status=[a-z_]*\|nodes=[0-9]*' | tr '\n' ' ')"
done
