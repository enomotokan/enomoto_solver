#!/bin/bash
# 1 スレッド・種 1 で perf record (cpu-clock、499 Hz、フレームポインタの呼び出しの鎖) をとり、自身の時間・子を含む時間の上位を出す。
# 使い方 (リポジトリの根から): bash analysis/hotspot_20261122/prof.sh 問題 秒 → analysis/hotspot_20261122/out/<問題>.{self,children}.txt
# perf.data は /tmp に置き、コミットしない。ビルドは README の「ビルド」(行番号表・フレームポインタつき) を使う。
set -u
P=/usr/lib/linux-tools-6.8.0-151/perf
D=analysis/hotspot_20261122; mkdir -p $D/out
prob=$1; tl=$2
data=/tmp/claude-0/perf_$prob.data
env $(env | grep -o '^ENOMOTO_[A-Z_0-9]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 \
  $P record -F 499 -g --call-graph fp -o $data -- python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$prob.mps --time-limit $tl --threads 1 \
  2>/dev/null | tail -1 > $D/out/$prob.result.json
$P report -i $data --no-children --sort symbol --stdio --percent-limit 0.3 2>/dev/null | grep -v '^#' | grep -v '^$' | grep -E '^ +[0-9]' | head -60 > $D/out/$prob.self.txt
$P report -i $data --children --sort symbol --stdio --percent-limit 1 -g none 2>/dev/null | grep -v '^#' | grep -v '^$' | grep -E '^ +[0-9]' | head -80 > $D/out/$prob.children.txt
cat $D/out/$prob.result.json | cut -c1-140
