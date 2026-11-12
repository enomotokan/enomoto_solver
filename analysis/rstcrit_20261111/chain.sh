#!/bin/sh
# 手元の順: cargo test の終わりを待って分担 3 (木の再スタートの ti・tt は TREEEFF に任せたので回さない)
cd "$(dirname "$0")"
while pgrep -f "cargo test" >/dev/null; do sleep 20; done
python run_share.py 3 --live >> share3.log 2>&1
