#!/bin/bash
# (b) サブ MIP が効く 6 問を 60 秒・4 スレッドで、既定 (def) と手当て (CFG、既定 h1 = ENOMOTO_MIP_SUBMIP_HIGHS=1) を交互に 2 回ずつ
cd "$(dirname "$0")"
H=${H:-1}; name=${NAME:-h1}
for k in 1 2; do
  for p in physiciansched6-2 mzzv42z neos-960392 30n20b8 bppc4-08 timtab1; do
    ./run_cfg.sh $p 60 def
    ./run_cfg.sh $p 60 $name ENOMOTO_MIP_SUBMIP_HIGHS=$H
  done
done | tee -a logs/batch_b.txt
echo BATCH_B DONE
