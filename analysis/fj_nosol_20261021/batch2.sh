#!/bin/bash
cd "$(dirname "$0")"
./run_cfg.sh uccase12 60 def ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh uccase12 60 ineq ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_P3_INEQ=1
./run_cfg.sh beasleyC3 60 def ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh beasleyC3 60 ineq ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_P3_INEQ=1
./run_cfg.sh beasleyC3 60 min8 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_MIN_TRIALS=8
./run_cfg.sh dcmulti 60 def ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh dcmulti 60 ineq ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_P3_INEQ=1
./run_cfg.sh roll3000 60 def ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh roll3000 60 ineq ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_P3_INEQ=1
./run_cfg.sh mc11 60 min8 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_MIN_TRIALS=8
./run_cfg.sh physiciansched6-2 60 e2ineq ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_FJ_P3_INEQ=1
echo SOLVES_DONE
cd ../.. && RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu cargo test --release --lib > analysis/fj_nosol_20261021/logs/cargo_test.txt 2>&1; echo "CARGO_TEST_EXIT=$?" >> analysis/fj_nosol_20261021/logs/cargo_test.txt
echo BATCH2_DONE
