#!/bin/bash
# perf record (cpu-clock 499 Hz、フレームポインタの鎖) を、スレッドの数を指定して取る (hotspot_20261122/prof.sh の写し)。
# 使い方 (リポジトリの根から、行番号表とフレームポインタつきのビルドで): bash analysis/proofprop_20261123/prof.sh 問題 秒 スレッド
# → analysis/proofprop_20261123/out/<問題>_t<スレッド>.self.txt。perf.data は scratchpad に置き、コミットしない。
set -u
P=/usr/lib/linux-tools-6.8.0-151/perf
D=analysis/proofprop_20261123; mkdir -p $D/out
prob=$1; tl=$2; th=$3
data=${PERF_DATA_DIR:-/tmp}/perf_${prob}_t$th.data
env $(env | grep -o '^ENOMOTO_[A-Z_0-9]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=$th ENOMOTO_T_MIP_SEED=1 \
  $P record -F 499 -g --call-graph fp -o $data -- python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$prob.mps --time-limit $tl --threads $th \
  2>/dev/null | tail -1 > $D/out/${prob}_t$th.result.json
$P report -i $data --no-children --sort symbol --stdio --percent-limit 0.3 2>/dev/null | grep -v '^#' | grep -v '^$' | grep -E '^ +[0-9]' | head -60 > $D/out/${prob}_t$th.self.txt
cut -c1-140 $D/out/${prob}_t$th.result.json
