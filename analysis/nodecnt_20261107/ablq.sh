#!/bin/bash
# アブレーションの待ち行列 (順に 1 つずつ)
cd "$(dirname "$0")"
while pgrep -f "abl.sh physiciansched" >/dev/null; do sleep 5; done
export CFGS="def nosb norestart noheur nonodecut nosym"
./abl.sh neos-3627168-kasai 150
./abl.sh neos-957323 150
./abl.sh mzzv11 250
./abl.sh csched008 300
./abl.sh unitcal_7 300
echo ALLDONE
