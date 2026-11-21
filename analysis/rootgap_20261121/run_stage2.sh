#!/bin/sh
# 2 段目: 1 段目で効いた群を手ごとに切る (上限 120 秒、ログつき)。
#  - 分離 (SEPAS: 分離の手 20 + 制約の分離 5): 根のカットで閉じる p200x1188c・exp-1-500-5-5・beasleyC3・mc11
#  - 前処理 (PRESOLS 16) と伝播 (PROPS 9): 前処理・伝播・再スタートが効く mc11・beasleyC3・app1-1
#  - ヒューリスティクス: RENS (mc11・beasleyC3・neos-1171448)、app1-1 で解を出した手 (conflictdiving) と LP の解
cd "$(dirname "$0")"
for p in p200x1188c exp-1-500-5-5 beasleyC3 mc11; do python scipab.py "$p" SEPAS --tl 120 --log; done
for p in mc11 beasleyC3 app1-1; do python scipab.py "$p" PRESOLS,PROPS --tl 120 --log; done
for p in mc11 beasleyC3 neos-1171448; do python scipab.py "$p" "p:heuristics/rens/freq=-1" --tl 120 --log; done
python scipab.py app1-1 "p:heuristics/conflictdiving/freq=-1,p:presolving/maxrestarts=0;propagating/probing/freq=-1,p:branching/relpscost/maxreliable=0;branching/relpscost/minreliable=0" --tl 120 --log
