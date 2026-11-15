#!/bin/bash
# q1.sh (600 秒の回) の後に 58 問 (60 秒) を rep 1・2 で
cd /home/user/enomoto_solver/analysis/submip_impr_20261106/B
while pgrep -f q1.sh > /dev/null; do sleep 30; done
python run58.py 1 def all
python run58.py 2 def all
