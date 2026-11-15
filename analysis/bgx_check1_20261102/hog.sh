#!/bin/sh
# 遅い機械の真似: CPU を n 本の空回しで埋めたまま run.py を回す。使い方: sh hog.sh n run.py の引数...
n=$1; shift
pids=""
for k in $(seq 1 $n); do python3 -c 'while 1: pass' & pids="$pids $!"; done
python run.py "$@"
kill $pids
