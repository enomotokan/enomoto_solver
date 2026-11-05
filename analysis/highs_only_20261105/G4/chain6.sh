#!/bin/bash
# ENOMOTO_MIP_HO4_CUT0=moved の検証 (G4.md 10 節)
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
python run.py ours mvsat 600 satellites2-60-fs
python run.py ours mv 600 neos-873061
python run.py ours mvt3 600 neos-3216931-puriri
python run.py ours premv 600 mzzv11
cp bench58.jsonl /tmp/claude-0/b58_before_mv.jsonl
python bench58.py def,mvb 0 > /tmp/claude-0/bench58c.out 2>&1
