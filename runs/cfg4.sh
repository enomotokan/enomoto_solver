#!/bin/bash
# 4 スレッド用 (このコンテナ専用)。使い方: cfg4.sh <tag> <time> [VAR=VAL ...]
tag=$1; T=$2; shift 2
cd "$(dirname "$0")"
mkdir -p t4
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 NTH=4 "$@" python run_enomoto.py $T > t4/e_$tag.log 2>&1
