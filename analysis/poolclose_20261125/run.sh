#!/bin/bash
# 試作 ENOMOTO_MIP_POOL_CLOSE=1 と既定の比べ (README.md)。1 スレッド・種 1〜3、3 本並べる。リポジトリの根から: bash analysis/poolclose_20261125/run.sh
# 結果は res.jsonl (終わった組は飛ばす)、ログは logs/ (gzip)。秒の上限: 終盤で下界が伸び悩む 8 問は 300 秒、neos5・pk1・mas76・mc11・beasleyC3 は 120 秒、
# rococoC10-001000 は 600 秒
cd "$(dirname "$0")/../.."
D=analysis/poolclose_20261125; mkdir -p $D/logs
one() { p=$1; s=$2; tag=$3; tl=$4
  kv="X=1"; [ $tag = pc ] && kv="ENOMOTO_MIP_POOL_CLOSE=1"
  grep -q "\"problem\": \"$p\", \"cfg\": \"$tag\", \"seed\": $s," $D/res.jsonl 2>/dev/null && return
  [ -f .miplib_cache/$p.mps ] || .venv/bin/python -c "import sys; sys.path.insert(0,'scripts'); import run_miplib_benchmark as rb; from pathlib import Path; rb.fetch('$p', Path('.miplib_cache'))" > /dev/null
  f=$D/logs/${p}_${tag}_s$s.log
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=$s $kv .venv/bin/python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit $tl --threads 1 > $f 2>&1
  r=$(grep '^{' $f | tail -1); st=$(grep 'POOL_CLOSE (tid 0)' $f | tail -1 | sed 's/"//g'); gzip -f $f
  echo "{\"problem\": \"$p\", \"cfg\": \"$tag\", \"seed\": $s, \"tl\": $tl, \"pc\": \"$st\", \"result\": ${r:-null}}" >> $D/res.jsonl
  echo "$p $tag s$s $(echo $r | cut -c1-110)"; }
export -f one; export D
{
for p in gmu-35-40 tr12-30 gen-ip002 gen-ip054 reblock115 icir97_tension sct2 neos-4738912-atrato; do for s in 1 2 3; do for t in def pc; do echo "$p $s $t 300"; done; done; done
for p in neos5 pk1 mas76 mc11 beasleyC3; do for s in 1 2 3; do for t in def pc; do echo "$p $s $t 120"; done; done; done
for s in 1 2 3; do for t in def pc; do echo "rococoC10-001000 $s $t 600"; done; done
} | xargs -P 3 -L 1 bash -c 'one $0 $1 $2 $3'
