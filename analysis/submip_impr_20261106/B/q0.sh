#!/bin/bash
# 既定 (ベースのビルド 5db4dd0f、試作の計数なし) の 1 回目
cd /home/user/enomoto_solver/analysis/submip_impr_20261106/B
for p in fiball neos-933966 trento1 nexp-150-20-8-5 thor50dday; do ./run1.sh def1 $p; done
