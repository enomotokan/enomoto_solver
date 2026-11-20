#!/bin/bash
# 群 2 の測り直し (今の既定、ビルドは PYTHONPATH で渡す)。1 スレッドは 4 本並べ、4 スレッドは 1 本ずつ。
cd "$(dirname "$0")"
O() { python3 ours.py "$@" --log > /dev/null 2>&1; }
O dano3_3 def --threads 1 & O dano3_5 def --threads 1 & O istanbul-no-cutoff def --threads 1 & O seymour def --threads 1 & wait
O supportcase42 def --threads 1 & O dano3_3 def --threads 1 --seed 2 & O dano3_5 def --threads 1 --seed 2 & O istanbul-no-cutoff def --threads 1 --seed 2 & wait
for p in dano3_3 dano3_5 istanbul-no-cutoff supportcase42 seymour; do O $p def; done
for p in dano3_3 dano3_5 istanbul-no-cutoff; do O $p def --seed 2; done
echo done > chain1.done
