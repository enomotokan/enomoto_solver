#!/bin/bash
# summ.sh <log>: 判定・根・最後の行・結果
f=$1
echo "== $f"
grep -c "relaxed root cut selection" $f | sed 's/^/fc_relax lines: /'
grep -E "Root cuts|cut loop extended" $f | tail -n 2
grep -E "background cuts: [0-9]" $f | grep -o "obj.*"
grep -E '^\s+[0-9.]+s\s' $f | tail -n 1
grep -o "RESULT [0-9.]*\|'status': '[a-z_]*'\|'objective': [-0-9.e+]*" $f | tr '\n' ' '; echo
