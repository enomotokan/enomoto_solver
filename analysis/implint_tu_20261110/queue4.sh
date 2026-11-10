#!/bin/bash
# queue3.sh の後: `e` (重複の除去と行の分割) で印が新しく付く 5 問で既定と bae を比べる (wheel は tu2v5、既定も同じ wheel)
cd "$(dirname "$0")"
until grep -q QUEUE_DONE /tmp/queue.log; do sleep 30; done
export IMPLINT_PKG=/home/user/pkgs/tu2v5
python3 run.py bE.jsonl def,tu2ae 1,2,3,4 60 mc11 sp150x300d binkar10_1 roll3000 mik-250-20-75-4
echo QUEUE4_DONE
