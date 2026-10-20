#!/bin/bash
# 使い方: batch.sh <名前> <mps> <秒> [ENV=VAL ...]  -> logs/<名前>.log
name=$1; mps=$2; T=$3; shift 3
env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python run_eno.py "$mps" "$T" > logs/$name.log 2>&1
echo "$name: $(grep -E 'new incumbent' logs/$name.log | tail -1 | cut -c1-120) | $(grep RESULT logs/$name.log | cut -c1-260)"
