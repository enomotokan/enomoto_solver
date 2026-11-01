#!/bin/bash
# 1 本ずつ順に回す (同時に走らせない)
cd "$(dirname "$0")"
./run.sh def2 ""; ./run.sh def3 ""; ./run.sh h1_2 1; ./run.sh h1_3 1
for c in p c s g t i h f; do ./run.sh "minus_${c}_1" "-$c"; done
for c in p c s g t i h f; do ./run.sh "only_${c}_1" "$c"; done
