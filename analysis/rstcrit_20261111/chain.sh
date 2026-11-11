#!/bin/sh
# 手元の順: Mittelmann の診断 -> 分担 5 -> Mittelmann の t
cd "$(dirname "$0")"
until grep -q "QUEUE DONE" queueM_diag.log; do sleep 30; done
python run_share.py 5 > share5.log 2>&1
python queue.py jobsM_t.txt > queueM_t.log 2>&1
