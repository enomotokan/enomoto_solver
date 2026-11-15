#!/bin/bash
# 5 回目: neos-960392 の遅い種で S1 の試作 RINSWIDE (根の RINS が空振りしたら固定を減らしてやり直す)。
# ビルドは e8e029c8 (/tmp/pyw2 を PYTHONPATH で使う)。neos-960392 の根の RINS は 1.2 秒かかるので秒の上限を 3 秒に
cd "$(dirname "$0")"
export PYTHONPATH=/tmp/pyw2
for s in 1 2 0; do
  python3 run1.py ours neos-960392 rw3_s$s 600 ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_S1_RINSWIDE=3 ENOMOTO_T_MIP_S1_RINSWIDE_SECS=3
done
python3 run1.py ours neos-960392 def2_s1 600 ENOMOTO_T_MIP_SEED=1
