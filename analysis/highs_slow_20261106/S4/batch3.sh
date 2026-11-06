#!/bin/bash
# batch2 の後: mzzv42z で再スタートの後の根のヒューリスティクスを全部回す既存の試作
cd "$(dirname "$0")"
while pgrep -f batch2.sh >/dev/null; do sleep 10; done
for s in 0 1 2; do python3 run1.py allh_s$s mzzv42z ours $s ENOMOTO_MIP_RESTART_ALL_HEURS=1; done
