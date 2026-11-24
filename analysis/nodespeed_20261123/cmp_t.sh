#!/bin/bash
# 9.6 節: 試作 ENOMOTO_MIP_NODEROWS_ADAPT=t を既定・r と比べる。リポジトリの根から: bash analysis/nodespeed_20261123/cmp_t.sh
# mc11 は 1・4 スレッド、ほかは 1 スレッド。種 1〜3、120 秒 (fhnw-binpack4-4 は 200 秒)。結果は cmp_t.jsonl (終わった組は飛ばす)、
# ログは logs/t_<問題>_<形>_t<スレッド>_s<種>.log.gz (コミットしない)。NRA_TRACE で 200 ノードごとの φ・f・ρ・比を出す
cd "$(dirname "$0")/../.."
D=analysis/nodespeed_20261123
run() { p=$1; th=$2; s=$3; tag=$4; kv=$5; tl=120; [ $p = fhnw-binpack4-4 ] && tl=200
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/cmp_t.jsonl 2>/dev/null && return
  f=$D/logs/t_${p}_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=200 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/cmp_t.jsonl
  echo "$p $tag t$th s$s $(echo $r | cut -c1-110)"; }
for th in 1 4; do for s in 1 2 3; do run mc11 $th $s t ENOMOTO_MIP_NODEROWS_ADAPT=t; done; done
for p in fhnw-binpack4-4 neos5 noswot swath1 roll3000 mik-250-20-75-4; do for s in 1 2 3; do
  run $p 1 $s def X=1; run $p 1 $s r ENOMOTO_MIP_NODEROWS_ADAPT=r; run $p 1 $s t ENOMOTO_MIP_NODEROWS_ADAPT=t
done; done
