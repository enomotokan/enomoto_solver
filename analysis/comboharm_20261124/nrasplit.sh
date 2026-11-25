#!/bin/bash
# 10.6 節 (親の依頼): nra の得が「行を外す側」と「選び直しのラウンドの側」のどちらから来るか。ti と、ti からラウンドの側をほぼ止めた形
# (tia = ti + 既存の係数 ENOMOTO_T_MIP_NODEROWS_ADAPT_TA=50: 持ち分 σ(50 ln 比) で、比 < 1 ならほぼ 0) と既定を比べる。種 1〜3、120 秒 (fhnw-binpack4-4 は 200 秒)。
# 1 スレッドは 3 本並べ、4 スレッドは 1 本ずつ。結果は res_nrasplit.jsonl、ログは logs_nrasplit/ (gzip、NRA_TRACE=200)
cd "$(dirname "$0")/../.."
D=analysis/comboharm_20261124; mkdir -p $D/logs_nrasplit
one() { p=$1; th=$2; s=$3; tag=$4; tl=120; [ $p = fhnw-binpack4-4 ] && tl=200; [ $p = neos-631710 ] && tl=600
  case $tag in def) kv="X=1";; ti) kv="ENOMOTO_MIP_NODEROWS_ADAPT=ti";; tia) kv="ENOMOTO_MIP_NODEROWS_ADAPT=ti ENOMOTO_T_MIP_NODEROWS_ADAPT_TA=50";; r0) kv="ENOMOTO_MIP_NODEROWS_ADAPT=r0";; r0k) kv="ENOMOTO_MIP_NODEROWS_ADAPT=r0k";; r0k2) kv="ENOMOTO_MIP_NODEROWS_ADAPT=r0k2";; r0k23) kv="ENOMOTO_MIP_NODEROWS_ADAPT=r0k23";; esac
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s," $D/res_nrasplit.jsonl 2>/dev/null && return
  f=$D/logs_nrasplit/${p}_${tag}_t${th}_s$s.log
  env ENOMOTO_MIP_NRA_TRACE=200 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=$s $kv .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads $th > $f 2>&1
  r=$(grep '^{' $f | tail -1); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"threads\": $th, \"seed\": $s, \"result\": ${r:-null}}" >> $D/res_nrasplit.jsonl
  echo "$p $tag t$th s$s $(echo $r | cut -c1-110)"; }
export -f one; export D
for p in mc11 beasleyC3 fhnw-binpack4-4 mik-250-20-75-4 roll3000; do for s in 1 2 3; do for tag in def ti tia r0 r0k; do echo "$p 1 $s $tag"; done; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
for p in neos-911970 neos5; do for s in 1 2 3; do for tag in def ti tia r0 r0k; do one $p 4 $s $tag; done; done; done
# 10.7 節: 判定 K の直し k が要るか (neos-631710、1 節。1 スレッド・600 秒)
for s in 1 2 3; do for tag in def r0 r0k; do echo "neos-631710 1 $s $tag"; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
# 10.8 節: 判定 K と持ち越しをカットプールで数える k2 (r0k2)
for s in 1 2 3; do for p in neos-631710 fhnw-binpack4-4 mik-250-20-75-4 roll3000 mc11; do echo "$p 1 $s r0k2"; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
# 10.9 節: 持ち越しを既定の年齢の上限に届いていない外した行に絞る k3 (r0k23)
for s in 1 2 3; do for p in neos-631710 mc11 fhnw-binpack4-4 mik-250-20-75-4 roll3000; do echo "$p 1 $s r0k23"; done; done | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
