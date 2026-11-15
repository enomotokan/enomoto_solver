#!/bin/bash
# 使い方: bench.sh 出力.jsonl 秒 "問題..." "種..." ラベル [環境変数=値 ...]
# 1 回ずつ順に解き、結果の JSON に問題・種・ラベルと cutconf の統計の行を足して追記する
out=$1; T=$2; probs=$3; seeds=$4; label=$5; shift 5
cd /home/user/enomoto_solver
for n in $probs; do for s in $seeds; do
  log=/tmp/cc_logs/${label}_${n}_s$s.log; mkdir -p /tmp/cc_logs
  r=$(env "$@" ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s timeout $((T*3+120)) python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$n.mps --time-limit $T --threads 4 2>$log | tail -1)
  [ -z "$r" ] && r='{"status": "crash"}'
  cc=$(grep "MIP: cutconf" $log | tail -1 | sed 's/"/ /g')
  python3 -c "import json,sys; d=json.loads(sys.argv[1]); d.update(problem=sys.argv[2], seed=int(sys.argv[3]), label=sys.argv[4], cc=sys.argv[5]); print(json.dumps(d))" "$r" $n $s $label "$cc" >> $out
done; done
