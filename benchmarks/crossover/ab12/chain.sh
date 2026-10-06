#!/bin/bash
# Mittelmann の小さめの 8 問の計測が終わるのを待ち、拡張を作り直してから第 12 回の比較を始める。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/mitt_small/done.txt ]; do sleep 60; done
[ -e benchmarks/crossover/ab12/built.txt ] || { . .venv/bin/activate && maturin develop --release > /dev/null 2>&1 && echo built > benchmarks/crossover/ab12/built.txt; }
benchmarks/crossover/ab12/run_ab.sh
