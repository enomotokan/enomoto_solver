#!/bin/bash
# 1 回目の試し: mzzv11・widden を既定と試作で (種 1、600 秒)
cd "$(dirname "$0")"
for p in mzzv11 neos-4722843-widden; do
  for c in ra def; do python3 run1.py $c $p 1 600; done
done
