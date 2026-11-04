#!/bin/sh
# 1 回の求解 (run_mem.py) と smaps の分類 (watch_proc.py) をいっしょに回す。使い方: sh run_watch.sh <名前> <問題> <秒> [heaptrack]
D=$(dirname "$0")
python "$D/run_mem.py" "$@" &
RP=$!
PY=""
while [ -z "$PY" ] && kill -0 $RP 2>/dev/null; do sleep 1; PY=$(pgrep -f "diag_worker.py .*$2.mps" | grep -v heaptrack | while read p; do [ "$(cat /proc/$p/comm)" = python ] && echo $p; done | head -1); done
[ -n "$PY" ] && python "$D/watch_proc.py" "$PY" "$D/watch_$1.jsonl" &
wait $RP
