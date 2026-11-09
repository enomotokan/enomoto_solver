#!/bin/bash
# 待ち行列: queue.txt の各行 (run1.py の引数) を上から 1 つずつ回す。済んだ行は /home/user/gap2work/done.txt に足し、再開時は飛ばす。
D=$(cd "$(dirname "$0")" && pwd)
Q=${1:-$D/queue.txt}
touch /home/user/gap2work/done.txt
while IFS= read -r line; do
  [ -z "$line" ] && continue
  grep -qxF "$line" /home/user/gap2work/done.txt && continue
  python "$D/run1.py" $line >> /home/user/gap2work/queue.out 2>&1
  echo "$line" >> /home/user/gap2work/done.txt
done < "$Q"
echo QUEUE_DONE >> /home/user/gap2work/queue.out
