#!/bin/bash
# 600 秒の待ち行列が終わったら 58 問 (4 スレッド・60 秒) を def・c0jc・tcap・jcx で rep 1・2。3 設定とも試作を足したビルド (/tmp/s6site) で回す
cd "$(dirname "$0")"
while pgrep -f "\./queue\.sh" >/dev/null; do sleep 10; done
export PYTHONPATH=/tmp/s6site
python3 bench58.py def,c0jc,tcap,jcx 1 > bench58_r1.out 2>&1
python3 bench58.py def,c0jc,tcap,jcx 2 > bench58_r2.out 2>&1
