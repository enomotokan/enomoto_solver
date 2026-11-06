#!/bin/bash
# 探り 2: 別枠 (q) の形 (60 秒、種 1)。batch0 の後に「足した点の候補をプールに入れない」「q」を足したビルド
cd "$(dirname "$0")"
for p in mc11 beasleyC3 mzzv42z neos859080; do
  ./run1.sh $p x1_${p}_def 4 1 60
  ./run1.sh $p x1_${p}_ocq 4 1 60 ENOMOTO_MIP_MPSEP=ocq
  ./run1.sh $p x1_${p}_oq 4 1 60 ENOMOTO_MIP_MPSEP=oq
  ./run1.sh $p x1_${p}_cq 4 1 60 ENOMOTO_MIP_MPSEP=cq
  ./run1.sh $p x1_${p}_oc 4 1 60 ENOMOTO_MIP_MPSEP=oc
done
