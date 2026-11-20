#!/bin/bash
# 遅れる 2 問 (rococoC10-001000・kasai) を 1 スレッド・ログつきで def と tsig で解く (nice 19、計測の邪魔を小さく)。
# 使い方: bash run1t.sh <PYTHONPATH (試作のビルド)> [設定の並び (既定 "def tsig")] [問題の並び]。設定 tsig = 値 1、tsigr = 値 r (根だけ)。ログは logs/<問題>_<設定>_s<種>.log.gz、結果は res1t.jsonl
cd "$(dirname "$0")"; D=$(pwd); WT=$(cd ../../../..; pwd)
for p in ${3:-rococoC10-001000 neos-3627168-kasai}; do for s in 3 4; do for c in ${2:-def tsig}; do
  [ -f logs/${p}_${c}_s${s}.log.gz ] && continue
  mkdir -p logs
  E="X=1"; [ $c = tsig ] && E="ENOMOTO_MIP_TAB_HIGHS_SIG=1"; [ $c = tsigr ] && E="ENOMOTO_MIP_TAB_HIGHS_SIG=r"
  t0=$(date +%s)
  out=$(cd $WT && env PYTHONPATH=$1 $E ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=$s nice -n 19 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit 600 --threads 1 2> >(awk -v t0=$t0 '{printf "[%6d] %s\n", systime()-t0, $0; fflush()}' | gzip > $D/logs/${p}_${c}_s${s}.log.gz) | tail -1)
  echo "{\"problem\": \"$p\", \"cfg\": \"$c\", \"seed\": $s, \"result\": $out}" >> res1t.jsonl
  echo "$p $c $s $out" | cut -c1-160
done; done; done
echo ALLDONE
