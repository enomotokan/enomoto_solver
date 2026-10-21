#!/bin/bash
# usage: run.sh tag T [ENV=VAL ...]   (ENOMOTO 1 回)
tag=$1; T=$2; shift 2
cd "$(dirname "$0")"
MPS=../../.miplib_cache/${PROB:-neos-2978193-inde}.mps
log=logs/${PROB:-neos-2978193-inde}_${tag}.log
env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python run1.py $MPS $T > $log 2>&1
echo "$tag $(grep RESULT $log) $(grep ^CPU $log)"
