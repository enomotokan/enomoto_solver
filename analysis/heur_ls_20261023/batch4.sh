#!/bin/bash
# 既定の測り直し (コンテナの再起動の後、batch3 と同じ機械で)。名前 def2
cd "$(dirname "$0")"
for rep in 1 2; do
for p in 30n20b8 neos-1456979 physiciansched6-2 roll3000 mzzv42z timtab1; do
  ./run_cfg.sh $p 60 def2 ENOMOTO_MIP_FJ_DEBUG=1
done
done
echo BATCH4 DONE
