#!/bin/bash
# 使い方: watch_at.sh 問題名 時間上限 開始秒 接頭辞 [環境変数 ...]
# watch.sh と同じく回し、起動から 開始秒 (wall、Python の起動・読み込みを含む) を過ぎたら終わるまで gdb で全スレッドのスタックを取り続ける (最大 10 回)。
# 期限の直後 (上限 + Python の読み込みの秒数) に何が走っているかを見る
cd /home/user/enomoto_solver
P=$1; T=$2; FROM=$3; TAG=$4; shift 4
L=/tmp/rep/${TAG}_${P}_${T}
s=$(date +%s.%N)
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" \
  python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads 4 > $L.log 2>&1 &
pid=$!
while kill -0 $pid 2>/dev/null && [ $(echo "$(date +%s.%N) - $s < $FROM" | bc) = 1 ]; do sleep 0.05; done
k=0
while kill -0 $pid 2>/dev/null && [ $k -lt 10 ]; do
  echo "=== stack $k at $(echo "$(date +%s.%N) - $s" | bc)s" >> $L.gdb
  timeout 20 gdb -p $pid -batch -ex "thread apply all bt 30" >> $L.gdb 2>&1
  k=$((k+1))
done
wait $pid
echo "$P $T wall $(echo "$(date +%s.%N) - $s" | bc) stacks $k $(tail -1 $L.log | cut -c1-140)" >> /tmp/rep/${TAG}_summary.txt
