#!/bin/bash
# jobs.txt の先頭の行 (run1.sh の引数) を 1 つずつ取り出して順に回す。highs.sh が終わるのを待ってから始める
cd $(dirname $0)
while pgrep -f "/bin/bash ./highs.sh" >/dev/null; do sleep 10; done
while true; do
  l=$(head -1 jobs.txt 2>/dev/null)
  if [ -z "$l" ]; then sleep 20; continue; fi
  sed -i 1d jobs.txt
  echo "$(date +%T) $l" >> q.log
  ./run1.sh $l
done
