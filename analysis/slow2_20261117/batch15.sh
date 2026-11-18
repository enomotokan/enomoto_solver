#!/bin/bash
# sw2 の noswot の害と doon の効きのログ (README 14 節)。traininstance6 の再現が終わってから回す
cd "$(dirname "$0")"
while pgrep -f "rep.py dbg2" >/dev/null; do sleep 20; done
for s in 1 2 3; do
  ./one.sh noswot ours $s 60 b15def
  ./one.sh noswot ours $s 60 b15sw2 ENOMOTO_T_MIP_SUBWAIT=2
done
./one.sh neos-2746589-doon ours 1 300 b15sw2 ENOMOTO_T_MIP_SUBWAIT=2
