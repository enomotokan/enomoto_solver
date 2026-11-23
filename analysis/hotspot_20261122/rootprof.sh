#!/bin/bash
# 根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1 の perf record と、根のカットのループの内訳 (rootloop.py・sepbreak.py) と CUTTK の行の数。
# 使い方 (リポジトリの根から): bash analysis/hotspot_20261122/rootprof.sh 問題 → analysis/hotspot_20261122/out/<問題>.root.txt
set -u
P=/usr/lib/linux-tools-6.8.0-151/perf
D=analysis/hotspot_20261122; prob=$1; data=/tmp/claude-0/perf_${prob}_root.data
env $(env | grep -o '^ENOMOTO_[A-Z_0-9]*' | sed 's/^/-u /') ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_NODE_LIMIT=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_CUTSTOP_LOG=1 \
  $P record -F 999 -g --call-graph fp -o $data -- python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$prob.mps --time-limit 120 --threads 1 \
  2>/tmp/claude-0/rootlog_$prob.log | tail -1 > /tmp/claude-0/rootres_$prob.json
{ echo "== $prob: $(cut -c1-120 /tmp/claude-0/rootres_$prob.json)"; echo "根のラウンド数 (CUTTK の行): $(grep -c '^CUTTK' /tmp/claude-0/rootlog_$prob.log)、止まり方: $(grep '^CUTSTOP' /tmp/claude-0/rootlog_$prob.log | grep -o '"reason":"[a-z_0-9]*"' | tr '\n' ' ')";
  python3 $D/rootloop.py $data; python3 $D/sepbreak.py $data; } > $D/out/$prob.root.txt
cat $D/out/$prob.root.txt
