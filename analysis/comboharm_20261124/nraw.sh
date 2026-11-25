#!/bin/bash
# 試作 ENOMOTO_MIP_NODEROWS_ADAPT の w (mc11.md 5 節): 既定・t・tw を、mc11 (1・4 スレッド) と nra の効いた fhnw-binpack4-4・mik-250-20-75-4・roll3000
# (1 スレッド) で、種 1〜3、120 秒 (fhnw-binpack4-4 は 200 秒) で比べる (nodespeed 9.6.2 節と同じ条件)。リポジトリの根から: bash analysis/comboharm_20261124/nraw.sh
# 1 スレッドの回は 3 本並べ、4 スレッドの回は 1 本ずつ。結果は res_nraw.jsonl (終わった組は飛ばす)、ログは logs_nraw/ (gzip、NRA_TRACE=200)
cd "$(dirname "$0")/../.."
D=analysis/comboharm_20261124; mkdir -p $D/logs_nraw
one() { p=$1; th=$2; s=$3; tag=$4; tl=120; [ $p = fhnw-binpack4-4 ] && tl=200
  case $tag in def) kv="X=1";; t) kv="ENOMOTO_MIP_NODEROWS_ADAPT=t";; tw) kv="ENOMOTO_MIP_NODEROWS_ADAPT=tw";; esac
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/res_nraw.jsonl 2>/dev/null && return
  f=$D/logs_nraw/${p}_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=200 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/res_nraw.jsonl
  echo "$p $tag t$th s$s $(echo $r | cut -c1-110)"; }
export -f one; export D
for p in mc11 fhnw-binpack4-4 mik-250-20-75-4 roll3000; do for s in 1 2 3; do for tag in def t tw; do echo "$p 1 $s $tag"; done; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
for s in 1 2 3; do for tag in def t tw; do one mc11 4 $s $tag; done; done
