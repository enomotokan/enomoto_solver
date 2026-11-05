#!/bin/bash
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
while pgrep -f chain2.sh > /dev/null; do sleep 20; done
python run.py ours c0d 600 satellites2-60-fs
python run.py ours bgnb 600 satellites2-60-fs
python run.py ours c0t3h 600 neos-3216931-puriri
python run.py ours c0h 600 neos-873061
