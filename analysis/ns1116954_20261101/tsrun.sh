#!/bin/bash
# 使い方: tsrun.sh <ログ> <mps> : run_eno.py を動かし、標準エラーの各行に始めからの経過秒を付けてログに書く (環境変数はそのまま渡る)
log=$1; mps=$2
D=$(cd "$(dirname "$0")" && pwd)
python "$D/run_eno.py" "$mps" 2> >(awk -v t0="$(date +%s.%N)" '{ "date +%s.%N" | getline t; close("date +%s.%N"); printf "[%6.2f] %s\n", t - t0, $0; fflush() }' > "$log")
sleep 0.5
