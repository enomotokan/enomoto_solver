#!/bin/bash
# 使い方: watch_at.sh 問題名 時間上限 開始秒 接頭辞 [環境変数 ...]
# 起動から 開始秒 (wall、MPS の読み込みを含む) を過ぎたら、終わるまで gdb で全スレッドのスタックを取り続ける (最大 12 回)。
# 期限の直後を見る用 (gdb が止める分だけ時刻は伸びる)。ログは logs/接頭辞_問題.log (.gdb)
cd /home/user/enomoto_solver
O=analysis/mittelmann_ready_20261103/overrun/logs
P=$1; T=$2; S=$3; TAG=$4; shift 4
L=$O/${TAG}_${P}
s=$(date +%s.%N)
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" \
  python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads 4 > $L.log 2>&1 &
pid=$!
while kill -0 $pid 2>/dev/null && [ $(echo "$(date +%s.%N) - $s < $S" | bc) = 1 ]; do sleep 0.1; done
k=0
while kill -0 $pid 2>/dev/null && [ $k -lt 12 ]; do
  echo "=== stack $k at $(echo "$(date +%s.%N) - $s" | bc)s" >> $L.gdb
  timeout 20 gdb -p $pid -batch -ex "thread apply all bt 30" >> $L.gdb 2>&1
  k=$((k+1))
done
wait $pid
echo "$P wall $(echo "$(date +%s.%N) - $s" | bc) stacks $k $(tail -1 $L.log | cut -c1-160)" >> $O/${TAG}_summary.txt
