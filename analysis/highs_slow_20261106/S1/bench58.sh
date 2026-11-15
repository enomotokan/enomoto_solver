#!/bin/bash
cd "$(dirname "$0")"
python3 bench58.py def,s1all 0
python3 bench58.py def,s1all 1
echo ALL_DONE
