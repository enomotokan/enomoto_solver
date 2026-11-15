#!/bin/bash
# 直した c で puriri の 1 回目を回し直す (chain2 の後)
cd "$(dirname "$0")"
while pgrep -f "chain2.sh" >/dev/null; do sleep 10; done
python run.py c 600 1 neos-3216931-puriri
