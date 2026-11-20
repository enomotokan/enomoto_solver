#!/bin/bash
# cutlen (3): 試作 ENOMOTO_MIP_CUTLEN=1 (8 ラウンド目の始めに r を数えて上限を 8〜25) を代表 8 問 × 種 1・2 で
cd "$(dirname "$0")"
for s in 1 2; do
for p in h80x6320d neos-911970 exp-1-500-5-5 neos-3381206-awhea p200x1188c qnet1 neos17 fiber; do
  ./one.sh $p ours $s 60 cl ENOMOTO_MIP_CUTLEN=1
done; done
