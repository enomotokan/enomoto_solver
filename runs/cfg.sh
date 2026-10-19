#!/bin/bash
# 使い方: cfg.sh <tag> <time> [VAR=VAL ...]
tag=$1; T=$2; shift 2
cd /home/user/wt/neos-911970/runs
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" /tmp/with_slot.sh python run_enomoto.py $T > e_$tag.log 2>&1
