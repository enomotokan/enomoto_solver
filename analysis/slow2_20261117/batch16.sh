#!/bin/bash
# noswot の def と sw2 を種 4〜13 で交互に (sw2 で SUBWAIT が一度も働かないのに遅いのが揺れかを見る、README 14 節)
cd "$(dirname "$0")"
while pgrep -f batch15.sh >/dev/null; do sleep 10; done
for s in 4 5 6 7 8 9 10 11 12 13; do
  if [ $((s % 2)) = 0 ]; then
    ./one.sh noswot ours $s 60 b16def; ./one.sh noswot ours $s 60 b16sw2 ENOMOTO_T_MIP_SUBWAIT=2
  else
    ./one.sh noswot ours $s 60 b16sw2 ENOMOTO_T_MIP_SUBWAIT=2; ./one.sh noswot ours $s 60 b16def
  fi
done
