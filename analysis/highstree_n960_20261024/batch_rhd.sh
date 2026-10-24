#!/bin/bash
# batch_rhd.sh: bench と同じ呼び出し (60 秒・4 スレッド) で ENOMOTO_MIP_RENS_HIGHS=dive を確かめる。結果は logs/batch_rhd.txt
cd "$(dirname "$0")/../.."
L=analysis/highstree_n960_20261024/logs
one() { # <問題> <名前> [ENV=VAL ...]
  p=$1; name=$2; shift 2
  env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit 60 --threads 4 > $L/b_${p}_$name.log 2>&1
  echo "$p $name $(tail -1 $L/b_${p}_$name.log)" >> $L/batch_rhd.txt
}
for i in 2 3 4 5; do one neos-960392 rhd$i ENOMOTO_MIP_RENS_HIGHS=dive; done
for p in physiciansched6-2 mzzv42z timtab1 30n20b8 app1-1 qiu h80x6320d; do
  for i in 1 2; do one $p rhd$i ENOMOTO_MIP_RENS_HIGHS=dive; done
done
for p in app1-1 qiu h80x6320d; do
  for i in 1 2; do one $p def$i; done
done
echo DONE >> $L/batch_rhd.txt
