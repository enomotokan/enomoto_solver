#!/bin/sh
# PGO の学習: build_var.sh pgogen の版で、測る 15 問と重ならない P の問題を 1 スレッド・種 1・上限 30 秒で解き、
# pgo/raw の *.profraw を rustup の llvm-profdata (LLVM の版を rustc に合わせる) で pgo/merged.profdata にまとめる。
set -e
D=$(cd "$(dirname "$0")" && pwd)
rm -rf "$D/pgo"; mkdir -p "$D/pgo/raw"
cd "$D"
PROBS=neos17,misc07,binkar10_1,eil33-2,graph20-20-1rand,neos-827175,hypothyroid-k1,p200x1188c,nu25-pr12,dcmulti TL=30 REPS=1 python run_var.py pgotrain pgogen
PD=$(ls ~/.rustup/toolchains/*/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata | head -n 1)
"$PD" merge -o "$D/pgo/merged.profdata" "$D"/pgo/raw/*.profraw
ls -la "$D/pgo/merged.profdata"; ls "$D"/pgo/raw | wc -l
