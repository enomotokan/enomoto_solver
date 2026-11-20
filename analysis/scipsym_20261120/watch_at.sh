#!/bin/bash
# 使い方: watch_at.sh 問題名 スレッド数 時間上限 開始秒 接頭辞 [環境変数 ...]
# analysis/overrun_crypt_20261101/watch_at.sh の写し (スレッド数を引数に、種 1、ログは logs/ に)。起動から 開始秒 (wall) を過ぎたら
# 終わるまで gdb で全スレッドのスタックを取り続ける (最大 10 回)。期限の直後に何が走っているかを見る
D=$(cd "$(dirname "$0")" && pwd); cd "$D/../.."
P=$1; TH=$2; T=$3; FROM=$4; TAG=$5; shift 5
L=$D/logs/${P}_${TAG}
s=$(date +%s.%N)
env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=$TH ENOMOTO_T_MIP_SEED=1 "$@" \
  python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $T --threads $TH > $L.log 2>&1 &
pid=$!
while kill -0 $pid 2>/dev/null && [ $(echo "$(date +%s.%N) - $s < $FROM" | bc) = 1 ]; do sleep 0.05; done
k=0
while kill -0 $pid 2>/dev/null && [ $k -lt 10 ]; do
  echo "=== stack $k at $(echo "$(date +%s.%N) - $s" | bc)s" >> $L.gdb
  timeout 20 gdb -p $pid -batch -ex "thread apply all bt 30" >> $L.gdb 2>&1
  k=$((k+1))
done
wait $pid
echo "$P $T wall $(echo "$(date +%s.%N) - $s" | bc) stacks $k $(tail -1 $L.log | cut -c1-160)"
