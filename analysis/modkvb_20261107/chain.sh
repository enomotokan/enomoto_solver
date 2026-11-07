#!/bin/bash
# queue_root.py が終わったら queue2.py を回す
cd "$(dirname "$0")"
while pgrep -f queue_root.py >/dev/null; do sleep 30; done
python queue2.py >> queue2.log 2>&1
