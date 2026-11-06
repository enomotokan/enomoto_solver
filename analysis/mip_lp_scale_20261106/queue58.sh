#!/bin/bash
# 58 問 × base/fix × 回 0・1 (済んだ組は飛ばすので、再起動の後はこれを流し直せばよい)
cd "$(dirname "$0")"
python3 bench58.py 0
python3 bench58.py 1
