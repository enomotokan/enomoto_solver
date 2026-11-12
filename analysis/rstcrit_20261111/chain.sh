#!/bin/sh
# 手元の順: Mittelmann の診断 -> 分担 5 -> Mittelmann の t
cd "$(dirname "$0")"
# (診断は済み)
python run_share.py 5 > share5.log 2>&1
python queue.py jobsM_t.txt > queueM_t.log 2>&1
python run_share.py 5 >> share5.log 2>&1  # rebase の事故で失った def p200x1188c 種 1 の 1 回
