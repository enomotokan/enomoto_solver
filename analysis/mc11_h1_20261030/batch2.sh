#!/bin/bash
# 効いていそうな形の繰り返しと、裏のカットを止めた形・裏のカットのログ
cd "$(dirname "$0")"
while pgrep -f batch1.sh >/dev/null; do sleep 5; done
./run.sh minus_s_2 "-s"; ./run.sh minus_s_3 "-s"
./run.sh only_s_2 "s"; ./run.sh only_s_3 "s"
./run.sh nobg_def_1 "" ENOMOTO_MIP_NO_BG_CUTS=1; ./run.sh nobg_h1_1 1 ENOMOTO_MIP_NO_BG_CUTS=1
./run.sh nobg_def_2 "" ENOMOTO_MIP_NO_BG_CUTS=1; ./run.sh nobg_h1_2 1 ENOMOTO_MIP_NO_BG_CUTS=1
./run.sh bglog_def "" ENOMOTO_MIP_BG_CUTS_LOG=1; ./run.sh bglog_h1 1 ENOMOTO_MIP_BG_CUTS_LOG=1
