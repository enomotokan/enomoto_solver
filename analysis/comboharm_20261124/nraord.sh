#!/bin/bash
# 11 節 (利用者の依頼): nra の外す行の並べ方 ENOMOTO_MIP_NRA_ORDER (a 年齢 / b 非零の多い順 / c 締まった回数 ÷ 非零 の低い順 / d 3 つの順位の和) を
# r0 (外す側だけ) で比べる。種 1〜3、120 秒 (fhnw-binpack4-4 は 200 秒)。1 スレッドは 3 本並べ、4 スレッドは 1 本ずつ。
# 行が密な問題として roll3000 (非零 1 行あたり約 6)・nw04 (約 150) を入れた。結果は res_nraord.jsonl、ログは logs_nraord/ (gzip、NRA_TRACE=200)
cd "$(dirname "$0")/../.."
D=analysis/comboharm_20261124; mkdir -p $D/logs_nraord
one() { p=$1; th=$2; s=$3; tag=$4; tl=120; [ $p = fhnw-binpack4-4 ] && tl=200
  kv="ENOMOTO_MIP_NODEROWS_ADAPT=r0 ENOMOTO_MIP_NRA_ORDER=$tag"
  [ $tag = def ] && kv="X=1"
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/res_nraord.jsonl 2>/dev/null && return
  f=$D/logs_nraord/${p}_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=200 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/res_nraord.jsonl
  echo "$p $tag t$th s$s $(echo $r | cut -c1-110)"; }
export -f one; export D
for p in mc11 beasleyC3 fhnw-binpack4-4 mik-250-20-75-4 roll3000 nw04; do for s in 1 2 3; do for tag in def a b c d; do echo "$p 1 $s $tag"; done; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
for p in neos-911970 neos5; do for s in 1 2 3; do for tag in def a b c d; do one $p 4 $s $tag; done; done; done
