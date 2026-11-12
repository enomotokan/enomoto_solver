#!/bin/sh
# 手元の順: 分担 5 の残り (rebase の事故で失った def p200x1188c 種 1) -> 分担 4
# (木の再スタートの ti・tt は TREEEFF に任せたので回さない)
cd "$(dirname "$0")"
python run_share.py 5 >> share5.log 2>&1
python run_share.py 4 > share4.log 2>&1
