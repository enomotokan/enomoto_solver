#!/bin/bash
# 使い方: hog.sh 本数 コマンド...
# CPU を食う裏の処理 (busy loop) を 本数 だけ走らせてコマンドを実行し、終わったら止める (遅い機械の代わり)
N=$1; shift
pids=()
for i in $(seq 1 $N); do (while :; do :; done) & pids+=($!); done
"$@"
kill ${pids[@]} 2>/dev/null
