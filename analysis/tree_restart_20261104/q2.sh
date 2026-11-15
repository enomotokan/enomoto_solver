#!/bin/bash
# 2 版 (重みを wopen と分けた後) の試作 (tr2) を、58 問 × 2 回と 600 秒の 5 問で回す。既定 (def) は前の回を使う (既定の動きは同じ)
cd "$(dirname "$0")/../.."
D=analysis/tree_restart_20261104
for rep in 1 2; do
  python3 $D/run58.py $rep tr2 >> $D/run58.log 2>&1
done
for p in swath3 neos-3627168-kasai csched008 fiball sct2; do
  echo "$(date +%T) start tr2 $p" >> $D/q.log
  python3 $D/run1.py tr2 $p 600 >> $D/q.log 2>&1
done
echo "$(date +%T) all done 2" >> $D/q.log
