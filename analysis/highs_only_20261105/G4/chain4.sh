#!/bin/bash
# 作業プロセスの再起動で chain2・chain3 が止まった後の並べ直し
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
python run.py ours c0t3 600 neos-3216931-puriri
python run.py ours c0d 600 satellites2-60-fs
python run.py ours pre 600 mzzv11
python run.py ours c0t3h 600 neos-3216931-puriri
python run.py ours c0h 600 neos-873061
python -I hdev.py ../../../.miplib_cache/satellites2-60-fs.mps 250 1 > /tmp/claude-0/sat_hdev.log 2>&1; mv /tmp/claude-0/sat_hdev.log logs/satellites2-60-fs_highs_dev1_250.log
python run.py ours bgnb 600 satellites2-60-fs
./prof_cutlp.sh
python run.py ours prec0 600 mzzv11
python run.py ours c0 600 mzzv11
python run.py ours c0dn 600 satellites2-60-fs
python run.py ours c0t3n 600 neos-3216931-puriri
python run.py ours c0n 600 neos-873061
./prof_cutlp.sh
python run.py ours ac 600 satellites2-60-fs
python run.py ours all 600 satellites2-60-fs
