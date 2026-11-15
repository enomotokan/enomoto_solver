#!/bin/bash
# 利用者の案の続き: par3kx (ENOMOTO_MIP_OPTFACE_PARCUTS=3kx、勝った写しが、負けた写しのカットのうち自分の LP の解を切るものを次のラウンドの候補に足す)。
# beasleyC3・neos859080 を種 1〜8・300 秒 (今のビルド)。結果は batch11.txt
cd "$(dirname "$0")"
for prob in beasleyC3 neos859080; do
  for s in 1 2 3 4 5 6 7 8; do
    ./run1.sh $prob ${prob}_par3kx_s$s 4 $s 300 ENOMOTO_MIP_OPTFACE_PARCUTS=3kx ENOMOTO_MIP_OPTFACE_RACE_LOG=1 >> batch11.txt
  done
done
echo ALLDONE >> batch11.txt
