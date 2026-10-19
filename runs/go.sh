#!/bin/bash
# go.sh <tag> <prob> <T> [ENV=VAL ...]  -> runs/<tag>_<prob>.log (ENOMOTO, 自分のビルド)
R=/home/user/wt/beasleyC3_mc11/runs; tag=$1; p=$2; T=$3; shift 3
env PYTHONPATH=/home/user/wt/beasleyC3_mc11/.pylib ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" /tmp/with_slot.sh python $R/run_eno.py $p $T > $R/${tag}_$p.log 2>&1
grep -E "^RESULT" $R/${tag}_$p.log | cut -c1-120
