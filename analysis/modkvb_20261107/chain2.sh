#!/bin/bash
# queue2.py が終わったら queue3.py を回す
cd "$(dirname "$0")"
while pgrep -f queue2.py >/dev/null; do sleep 30; done
python queue3.py >> queue3.log 2>&1
