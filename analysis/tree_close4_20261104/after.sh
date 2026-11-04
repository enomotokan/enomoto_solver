#!/bin/bash
# 待ち行列 (jobs.txt) が空になり、600 秒の回が終わったら q.sh を止めて 58 問を回す
cd $(dirname $0)
while [ -s jobs.txt ] || pgrep -f "run1.sh" >/dev/null; do sleep 20; done
pkill -f "/bin/bash ./q.sh"
python run58.py def t4r > run58.log 2>&1
