#!/bin/bash
# batch2 の 2 回目 (コンテナの再起動で止まった分)
cd "$(dirname "$0")"
for p in 30n20b8 neos-1456979 physiciansched6-2 roll3000 mzzv42z timtab1; do
  ./run_cfg.sh $p 60 fjc ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_CHAIN=1
done
echo BATCH2 DONE
