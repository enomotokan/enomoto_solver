#!/bin/bash
# 計測を 1 つずつ順に回す (同時に 2 つは回さない)
cd "$(dirname "$0")/../.."
D=analysis/tree_restart_20261104
for c in def tr trp; do
  for p in swath3 neos-3627168-kasai csched008 fiball sct2; do
    echo "$(date +%T) start $c $p" >> $D/q.log
    python3 $D/run1.py $c $p 600 >> $D/q.log 2>&1
  done
done
for rep in 1 2; do
  python3 $D/run58.py $rep def tr >> $D/run58.log 2>&1
done
echo "$(date +%T) all done" >> $D/q.log
