#!/bin/bash
# q3.sh・q4.sh の後: 58 問・60 秒 (既定と ho2)。q2.sh の中の 58 問は run58.py の道のりの誤りで動かなかったので回し直す
cd "$(dirname "$0")"
while pgrep -f "q2.sh|q3.sh|q4.sh|q600.sh" > /dev/null; do sleep 10; done
python run58.py 1 def ho2 > run58.log 2>&1
