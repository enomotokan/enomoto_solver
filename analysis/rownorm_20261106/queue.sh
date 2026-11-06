#!/bin/bash
# 計測の待ち行列 (同時に 1 つずつ): fastxgemm 600 秒 (既定・ROWNORM を交互に 2 回ずつ、ROWNORM だけ (SYMNORM=0) を 1 回)、58 問 2 回、誤答の検査
cd /home/user/enomoto_solver/analysis/rownorm_20261106
P=fastxgemm-n2r6s0t2
./run1.sh $P def1
./run1.sh $P rn1 ENOMOTO_MIP_ROWNORM=1
./run1.sh $P rn2 ENOMOTO_MIP_ROWNORM=1
./run1.sh $P def2
./run1.sh $P rnnosym1 ENOMOTO_MIP_HO3_SYMNORM=0 ENOMOTO_MIP_ROWNORM=1
echo fastx-done >> queue.log
python bench58.py rownorm 0 >> bench58.log 2>&1
python bench58.py rownorm 1 >> bench58.log 2>&1
cp /tmp/claude-0/-home-user-enomoto-solver/a5f7a634-6cc1-5fca-8eb4-42ca5e3d5078/scratchpad/runs/bench58.jsonl .
echo bench-done >> queue.log
cd /home/user/enomoto_solver
U="env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /')"
$U ENOMOTO_MIP_ROWNORM=1 ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > analysis/rownorm_20261106/fuzz_tiny_rownorm.txt 2>&1
$U ENOMOTO_MIP_ROWNORM=1 ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300 > analysis/rownorm_20261106/fuzz_highs_rownorm.txt 2>&1
echo fuzz-done >> analysis/rownorm_20261106/queue.log
