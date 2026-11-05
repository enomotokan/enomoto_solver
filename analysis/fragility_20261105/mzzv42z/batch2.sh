#!/bin/bash
# batch1 の後に: 試作入りの wheel を入れ、ENOMOTO_MIP_ROBUST_STALL で種 0〜9 を回し、58 問 (def・rob) を回す
cd "$(dirname "$0")"  # (計測のときは analysis/fragility_20261105/ に置いていた)
while pgrep -f batch1.sh > /dev/null; do sleep 10; done
(cd ../.. && rm -rf /tmp/whl && maturin build --release -o /tmp/whl > /dev/null 2>&1 && pip install -q --force-reinstall --no-deps /tmp/whl/*.whl > /dev/null 2>&1) || { echo build failed; exit 1; }
for s in 0 1 2 3 4 5 6 7 8 9; do ./run1.sh rob_t4_s${s} $s 4 ENOMOTO_MIP_ROBUST_STALL=1; done
python run58.py def rob
echo all done
