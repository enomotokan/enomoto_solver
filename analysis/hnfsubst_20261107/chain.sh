#!/bin/bash
# 計測の順: 新しい wheel の sub の 60 秒ふるい → 600 秒 (6 問 × def,hnf,sub × 種 1・2) → 58 問 (60 秒、def,sub、種 1・2)
cd "$(dirname "$0")"
python run.py screen60b.jsonl 60 sub 1 @hits
python run.py long600.jsonl 600 def,sub,hnf 1,2 mzzv11 rococoB10-011000 rococoC10-001000 traininstance6 traininstance2 brazil3
python run.py p58.jsonl 60 def,sub 1,2 @p58
