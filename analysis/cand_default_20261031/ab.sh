#!/bin/bash
# 使い方: ab.sh 回 問題...
# 旧ビルド (/tmp/oldwhl、6bfcc89d) に旧 cand の環境変数を付けた形 (old) と、新ビルド (/tmp/newwhl) を環境変数なしで回した形 (new) を交互に回す。
# 4 スレッド・60 秒、ENOMOTO_MIP_LOG=1。ログは /tmp/rep/{old,new}R_問題_60_0.log
cd /home/user/enomoto_solver
A=analysis/cand_default_20261031
R=$1; shift
for p in "$@"; do
  pip install -q --force-reinstall --no-deps /tmp/oldwhl/*.whl 2>/dev/null
  $A/watch.sh $p 60 1 old$R $(cat $A/cand.env)
  pip install -q --force-reinstall --no-deps /tmp/newwhl/*.whl 2>/dev/null
  $A/watch.sh $p 60 1 new$R
done
