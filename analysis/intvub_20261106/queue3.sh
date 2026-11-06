#!/bin/bash
# キュー 3 (2026-11-06): 600 秒の比べ (long600.jsonl)。cargo test が終わるのを待ってから
cd "$(dirname "$0")"
while pgrep -f "cargo test" > /dev/null; do sleep 20; done
python run.py def neos859080 600 1 long600.jsonl
python run.py iv1 mzzv42z 600 1 long600.jsonl
python run.py def mzzv42z 600 1 long600.jsonl
