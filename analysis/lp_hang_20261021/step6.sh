#!/bin/bash
# 修正 (cleanup・polish・主単体法の引き継ぎのピボットで DSE 重みを更新) の確認
cd "$(dirname "$0")"; D=$(pwd)
cd ../.. && maturin build --release -o dist 2>&1 | grep -E "^error|Built" && pip install --no-deps --force-reinstall dist/*.whl 2>&1 | grep -i "^success"
cd "$D"
E="ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_LP=1"
env $E ENOMOTO_MIP_DEBUG_DSE_CHECK=1 ENOMOTO_DEBUG_EXT_ITERS=1 python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_e2c_fix2_dsecheck_16.log 2>&1
echo "== fix2 + DSE_CHECK: $(tail -1 logs/physiciansched6-2_e2c_fix2_dsecheck_16.log | cut -c1-150)"
grep "DSE_CHECK after solve\|primal_handoff_iters\|MIP: cut round .*: LP " logs/physiciansched6-2_e2c_fix2_dsecheck_16.log | head -40 | cut -c1-150
env $E python3 eno_run.py ../../.miplib_cache/physiciansched6-2.mps 60 4 > logs/physiciansched6-2_e2c_fix2_17.log 2>&1
echo "== fix2 (E2C): $(tail -1 logs/physiciansched6-2_e2c_fix2_17.log | cut -c1-150)"
grep "MIP: cut round .*: .* cuts (of\|continued after\|removed the cuts" logs/physiciansched6-2_e2c_fix2_17.log | cut -c1-140
cd ../.. && export RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu && cargo test --release --lib 2>&1 | grep "^test result\|FAILED\|panicked" ; cd "$D"
run() { p=$1; name=$2; shift 2; n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
  env ENOMOTO_MIP_THREADS=4 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps 60 4 > logs/${p}_${name}_${n}.log 2>&1
  echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log)"; }
for p in physiciansched6-2 10teams qnet1 misc07 neos-1456979 nw04 mik-250-20-75-4 neos-911970 binkar10_1 neos5 air03; do
  run $p fix2 ENOMOTO_MIP_LOG=1
  run $p nopolishdse ENOMOTO_MIP_LOG=1 ENOMOTO_NO_POLISH_DSE=1
done > logs/side2.out 2>&1
echo "== side2 done"
