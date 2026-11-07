#!/bin/bash
# queue1 の後に queue2 を回す
cd "$(dirname "$0")"
while pgrep -f queue1.py >/dev/null; do sleep 30; done
python3 queue2.py >> queue2.log 2>&1
