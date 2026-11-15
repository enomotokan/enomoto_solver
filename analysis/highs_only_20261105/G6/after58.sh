#!/bin/bash
# 待ち行列 (q.sh) が終わってから 58 問を既定と試作で回す
cd $(dirname $0)
while pgrep -f "./q.sh" >/dev/null; do sleep 20; done
python run58.py def ho6 > run58.log 2>&1
