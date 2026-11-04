#!/bin/sh
# tree_close3: 新しいビルドで既定と t3 (試作 4 つ) を 6 問・600 秒で交互に、続けて 58 問・60 秒 (既定・t3)
cd "$(dirname "$0")"
export TREE3_TMP=/tmp/claude-0/t3tmp
while kill -0 1195 2>/dev/null; do sleep 20; done
for p in neos-3216931-puriri satellites2-60-fs tbfp-network neos-873061 mzzv11 net12; do
  python run.py ours t3 600 $p
  python run.py ours def2 600 $p
done
python bench58.py def,t3 0
