#!/bin/sh
# Mittelmann の計測の終わりを待つ (1 回の通知用)
cd "$(dirname "$0")"
while pgrep -f "queue.py jobsM.txt" >/dev/null; do sleep 60; done
wc -l < resM.live.jsonl
