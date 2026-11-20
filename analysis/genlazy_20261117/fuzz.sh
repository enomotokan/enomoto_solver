#!/bin/bash
# GEN_LAZY の誤答の確かめ (HiGHS と照合): bash fuzz.sh 名前 台本 問題数 開始seed [環境変数...]
# 結果を fuzz_名前.txt に、外した・戻した数の要約を足す。リポジトリの根から .venv の python で回す。
cd "$(dirname "$0")/../.."
name=$1; scr=$2; n=$3; s0=$4; shift 4
err=$(mktemp)
env ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_GEN_LAZY=1 "$@" .venv/bin/python $scr $n $s0 2>$err | grep -E "^bad|^seed" > analysis/genlazy_20261117/fuzz_$name.txt
{
  echo "env: $*"
  echo "problems with drops: $(grep -c 'MIP: GEN_LAZY:' $err)"
  echo "stats lines with restores: $(grep -Ec 'checks [0-9]+ restored [1-9]' $err)  / $(grep -c 'GEN_LAZY stats' $err)"
  echo "total restored: $(grep -Eo 'checks [0-9]+ restored [0-9]+' $err | awk '{s+=$4} END {print s}')  re-restored: $(grep -Eo ' re-restored [0-9]+' $err | awk '{s+=$2} END {print s}')"
  echo "restored from strong branching children: $(grep -Eo 'sb restored [0-9]+' $err | awk '{s+=$3} END {print s}')"
  echo "restored by kind (clq,one,vb,knap,int,mix,cont): $(grep -o 'by kind \[[0-9, ]*\]' $err | sed 's/by kind //;s/[][]//g' | awk -F', ' '{for(i=1;i<=7;i++) s[i]+=$i} END {for(i=1;i<=7;i++) printf "%d ", s[i]}')"
} >> analysis/genlazy_20261117/fuzz_$name.txt
rm -f $err
tail -6 analysis/genlazy_20261117/fuzz_$name.txt
