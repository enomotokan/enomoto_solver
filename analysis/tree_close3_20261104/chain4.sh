#!/bin/sh
# chain3 の後: DIVECAP を足した t3 で puriri・satellites をやり直す
cd "$(dirname "$0")"
export TREE3_TMP=/tmp/claude-0/t3tmp
while pgrep -f "chain3.sh" >/dev/null; do sleep 30; done
python run.py ours t3 600 neos-3216931-puriri satellites2-60-fs
