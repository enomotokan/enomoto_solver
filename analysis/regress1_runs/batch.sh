#!/bin/bash
# batch.sh <tag> <pylib|sys> <T> <problems...>   ; extra env passed through
tag=$1; lib=$2; T=$3; shift 3
for p in "$@"; do
  /tmp/with_slot.sh /home/user/wt/regress1-scripts/run.sh $lib $p $T 4 /home/user/wt/regress1/analysis/regress1_runs/${tag}_${p}_t4.log
done
