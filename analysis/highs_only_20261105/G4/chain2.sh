#!/bin/bash
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
while pgrep -f chain1.sh > /dev/null; do sleep 20; done
python run.py ours c0 600 satellites2-60-fs neos-873061
python run.py ours c0t3 600 neos-3216931-puriri
python run.py ours pre 600 mzzv11
python -I hdev.py ../../../.miplib_cache/satellites2-60-fs.mps 250 1 > logs/satellites2-60-fs_highs_dev1_250.log 2>&1
