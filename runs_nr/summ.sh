#!/bin/bash
# RESTDIAG を出どころ/種類ごとに全スレッドで合計する。使い方: summ.sh <err ファイル>
grep RESTDIAG "$1" | sed 's/ | /\n   /g' | grep -E "^   (local|pool|dive)" | awk '{k=$1; n[k]+=$3; it[k]+=$3*$5; ms[k]+=$3*$7; ad[k]+=$3*$9; lo[k]+=$3*$11; re[k]+=$3*$13} END {for (k in n) printf "   %-14s n %5d it %7.1f ms %6.2f add %6.1f lost %5.1f readd %5.1f\n", k, n[k], it[k]/n[k], ms[k]/n[k], ad[k]/n[k], lo[k]/n[k], re[k]/n[k]}' | sort
grep RESTDIAG "$1" | awk '{s+=$7} END {printf "   restore_secs total %.2f\n", s}'
