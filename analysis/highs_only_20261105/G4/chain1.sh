#!/bin/bash
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
python run.py ours def 600 satellites2-60-fs neos-3216931-puriri mzzv11 neos-873061
python run.py highs def 600 satellites2-60-fs neos-3216931-puriri mzzv11 neos-873061
