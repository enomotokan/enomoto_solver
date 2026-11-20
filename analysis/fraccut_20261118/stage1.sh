#!/bin/bash
# 段 1: 根が退化している問題で、d = 既定 + 診断、o = 非整数の列の数の減りで停滞を打ち消さない (早く止める)、
# c = 非整数の度合いが 1 割以上減ったラウンドも停滞と数えない (長く続ける)。どれも診断つき。種 1・2、120 秒、4 スレッド。
cd "$(dirname "$0")"
for s in 1 2; do for p in roll3000 unitcal_7 mc11 sp150x300d neos-1445765 beasleyC3 p200x1188c mik-250-20-75-4 misc07; do
  for c in d o c; do
    [ -f logs/${p}_${c}_s${s}.log ] && continue
    ./one.sh $p ours $s 120 $c ENOMOTO_MIP_FRACCUT=$c$([ $c != d ] && echo d) ENOMOTO_MIP_DEBUG_CUTEXIT=1 | sed "s/^/$p $c $s /"
  done
done; done
