#!/bin/bash
# 58 問の害の確認 (batch6 の後): def・s4c・rfeqff を 4 スレッド・60 秒で 2 回 (種 = 回)
cd "$(dirname "$0")"
while pgrep -f batch6.sh >/dev/null; do sleep 10; done
export BENCH58_OUT=/tmp/claude-0/-home-user-enomoto-solver/32ea3e9e-13ab-5c94-ac9c-4652ac9834b7/scratchpad/runs/bench58.jsonl
python3 bench58.py def,s4c,rfeqff 1
python3 bench58.py def,s4c,rfeqff 2
