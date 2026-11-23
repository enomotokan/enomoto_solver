#!/bin/sh
# 1 回だけ解く: sh one.sh <名前> <スレッド> <秒> [VAR=値 ...]  (ログは logs/<名前>.log、結果の 1 行を標準出力へ)
# fhnw-binpack4-4・種 1・ENOMOTO_MIP_LOG=1。リポジトリの根で回す
name=$1; th=$2; tl=$3; shift 3
env "$@" ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/fhnw-binpack4-4.mps --time-limit $tl --threads $th 2> analysis/nodespeed_20261123/logs/$name.log | tail -1 | sed "s/^/$name /"
grep "node diag" analysis/nodespeed_20261123/logs/$name.log | sed "s/^/$name /"
