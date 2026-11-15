#!/bin/bash
# 担当の 4 問 (600 秒 x 2 回、既定と試作 c を交互に)。chain1 の後に回す
cd "$(dirname "$0")"
while pgrep -f chain1.sh >/dev/null; do sleep 10; done
for rep in 1 2; do
  for p in neos-873061 neos-3216931-puriri satellites2-60-fs mzzv11; do
    if [ $rep = 1 ]; then o="def c"; else o="c def"; fi
    for cfg in $o; do python run.py $cfg 600 $rep $p; done
  done
done
