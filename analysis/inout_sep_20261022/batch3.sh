#!/bin/bash
# 副作用: FACE_SEP が絡む固定費用型 (beasleyC3・h80x6320d) と 10teams・noswot・misc07 など、def / face を 3 回ずつ
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in beasleyC3 h80x6320d 10teams noswot misc07 rout neos5 mik-250-20-75-4 qnet1 timtab1; do
    for c in def face; do $D/erun.sh $p $c $k; done
  done
done
