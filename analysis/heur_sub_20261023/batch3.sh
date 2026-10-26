#!/bin/bash
# 軽い問題で悪くならないか (既定と 5 つ全部、2 回ずつ)
cd "$(dirname "$0")"; source configs.sh
for r in 1 2; do for p in misc07 10teams mik-250-20-75-4 qnet1; do for c in base all5; do
  ./run_cfg.sh $p 60 $c ${CFG[$c]}
done; done; done
