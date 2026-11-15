#!/bin/bash
# queue2 の後に queue3 を回す
cd "$(dirname "$0")"
while pgrep -f "queue2.py|chain.sh" >/dev/null; do sleep 30; done
python3 queue3.py >> queue3.log 2>&1
