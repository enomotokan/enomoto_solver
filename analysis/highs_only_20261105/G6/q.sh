#!/bin/bash
# jobs.txt の先頭の行 (run1.sh の引数) を 1 つずつ取り出して順に回す。空になったら終わる。ビルド中は待つ
cd $(dirname $0)
while true; do
  while pgrep -f "maturin build" >/dev/null || pgrep -x cargo >/dev/null || pgrep -x ninja >/dev/null || [ -e /tmp/claude-0/hold ] || ! grep -q "^done" /tmp/claude-0/build.log; do sleep 10; done
  l=$(head -1 jobs.txt 2>/dev/null)
  if [ -z "$l" ]; then exit 0; fi
  sed -i 1d jobs.txt
  echo "$(date +%T) $l" >> q.log
  case "$l" in H\ *) ./hinst.sh ${l#H } ;; *) ./run1.sh $l ;; esac
done
