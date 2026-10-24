#!/bin/bash
# rule_only.sh <ENOMOTO の dump> <名前> <有効にする規則のビット...>: そのビットだけ有効にした HiGHS の前処理 → ENOMOTO (既定、14 秒) の根の LP
cd "$(dirname "$0")"
src=$1; tag=$2; shift 2
all=0; for b in $(seq 6 19); do all=$((all | (1<<b))); done
on=0; for b in "$@"; do on=$((on | (1<<b))); done
off=$((all & ~on))
python3 highs_presolve_dump.py $src /tmp/claude-0/${tag}.mps presolve_rule_off=$off 2>&1 | grep PRESOLVED | sed "s/^/[$tag on=$*] /"
ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 python3 eno_run.py /tmp/claude-0/${tag}.mps 14 4 > logs/${tag}.log 2>&1
grep -E "^Presolve:|clique extension|MIP: root LP |cut round 0: [0-9]+ cuts \(" logs/${tag}.log | head -4 | sed "s/^/   /"
