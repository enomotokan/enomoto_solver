#!/bin/bash
# 段 2: 両問題 × {当方の既定, HiGHS} × 種 1、600 秒、ログつき (ソルバーは同時に 1 つ)
D=$(cd "$(dirname "$0")" && pwd)
for p in mzzv42z unitcal_7; do
  "$D/one.sh" $p ours 1 600 def
  "$D/one.sh" $p highs 1 600 highs
done
