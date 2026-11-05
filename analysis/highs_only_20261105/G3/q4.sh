#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
while pgrep -f "q[123].sh" >/dev/null; do sleep 20; done
python bench58.py def,ho3 0 > bench58.log 2>&1
