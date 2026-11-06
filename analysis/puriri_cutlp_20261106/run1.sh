#!/bin/bash
# 書き出したカットの LP を 1 回解く: run1.sh <名前> [環境変数 ...]。出力は analysis/puriri_cutlp_20261106/v_<名前>.txt
# 既定で根の最適基底から、40 秒の上限、ENOMOTO_PROF_PHASES_EXT=1・ENOMOTO_SIMPLEX_PIVLOG=1
cd "$(dirname "$0")/../.."
name=$1; shift
T=$(ls -t target/release/deps/_core-* | grep -v '\.d$' | head -1)
env LD_LIBRARY_PATH=/usr/local/lib CUTLP=${CUTLP:-/tmp/dump/cutlp_r0_TimeLimit} CUTLP_TIME=${CUTLP_TIME:-40} ENOMOTO_PROF_PHASES_EXT=1 ENOMOTO_PROF_TRIANGULAR=1 ENOMOTO_SIMPLEX_PIVLOG=1 "$@" \
  $T cut_lp_debug --ignored --nocapture > analysis/puriri_cutlp_20261106/v_$name.txt 2>&1
grep -E "^PROF_PHASES_EXT wall=[0-9]{3,}|^CUTLP: (rep|pivlog)|pivot_threshold=|refactor  " analysis/puriri_cutlp_20261106/v_$name.txt | head -8 | sed "s/^/[$name] /"
