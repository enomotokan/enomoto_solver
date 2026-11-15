#!/bin/bash
# PAPER7 の計測の並び (58 問の後に続けて回す。どれも途中から再開できる)
cd "$(dirname "$0")"
export IMPLINT_PKG=/home/user/pkgs/tu2v3
while pgrep -f "run.py b58.jsonl" >/dev/null; do sleep 30; done
# (1) 印が変わる 58 問の部分で、既存の試作 tu・カットに全部整数とする tu2・刻みも使う tu2as
python3 run.py b58x.jsonl tu,tu2 1,2 60 exp-1-500-5-5 physiciansched6-2 flugpl
python3 run.py b58x.jsonl tu2 1,2 60 @aff58
python3 run.py b58x.jsonl tu2as 1,2 60 exp-1-500-5-5 pk1 gr4x6 khb05250 assign1-5-8
# (2) 47 問のうち印が変わる問題 (600 秒)
python3 run.py b47.jsonl def,tu2a 1,2 600 @aff47
python3 run.py b47.jsonl tu2as 1,2 600 csched008 tr12-30
echo QUEUE_DONE
