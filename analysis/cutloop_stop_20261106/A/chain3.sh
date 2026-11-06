#!/bin/bash
# 58 問 (4 スレッド・60 秒、既定と試作 c、rep 1・2)。chain2 の後に回す
cd "$(dirname "$0")"
while pgrep -f "chain[12].sh" >/dev/null; do sleep 10; done
python bench58.py def,c 1
python bench58.py def,c 2
