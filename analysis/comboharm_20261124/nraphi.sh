#!/bin/bash
# 10.11 節 (親の依頼): φ の自己強化の直し NODEROWS_ADAPT の p (r0k23p) を、既定・r0k23 と同じビルド (磨きの既定化の後) で比べる。
# neos-911970 は 1 スレッド種 1〜3・4 スレッド種 1〜6、ほかは 1 スレッド (neos5 は 4 スレッド) 種 1〜3。120 秒 (fhnw 200 秒、neos-631710 600 秒)。
# 1 スレッドは 3 本並べ、4 スレッドは 1 本ずつ。結果は res_nraphi.jsonl、ログは logs_nraphi/ (gzip、NRA_TRACE=1000)
cd "$(dirname "$0")/../.."
D=analysis/comboharm_20261124; mkdir -p $D/logs_nraphi
one() { p=$1; th=$2; s=$3; tag=$4; tl=120; [ $p = fhnw-binpack4-4 ] && tl=200; [ $p = neos-631710 ] && tl=600
  case $tag in def) kv="X=1";; *) kv="ENOMOTO_MIP_NODEROWS_ADAPT=$tag";; esac
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/res_nraphi.jsonl 2>/dev/null && return
  f=$D/logs_nraphi/${p}_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=1000 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/res_nraphi.jsonl
  echo "$p $tag t$th s$s $(echo $r | cut -c1-110)"; }
export -f one; export D
for p in neos-911970 mc11 beasleyC3 fhnw-binpack4-4 mik-250-20-75-4 roll3000 neos-631710; do for s in 1 2 3; do for tag in def r0k23 r0k23p; do echo "$p 1 $s $tag"; done; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
for p in neos-911970; do for s in 1 2 3 4 5 6; do for tag in def r0k23 r0k23p; do one $p 4 $s $tag; done; done; done
for s in 1 2 3; do for tag in def r0k23 r0k23p; do one neos5 4 $s $tag; done; done
