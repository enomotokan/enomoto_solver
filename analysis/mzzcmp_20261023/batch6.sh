#!/bin/bash
# 6 回目: HiGHS の前処理の規則を 1 つずつ止める (Enumeration = 65536、Probing = 32768、支配列は Dual fixing と別なので止めない)
cd "$(dirname "$0")"
./run_highs.sh mzzv42z 60 noenum presolve_rule_off=65536
./run_highs.sh mzzv42z 60 noprobe presolve_rule_off=32768
