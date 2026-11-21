#!/bin/sh
# 直す案 5・6・7 の試作を 1 問ずつ回す (2026-11-21、600 秒・4 スレッド・種 1、ログは logs/p<案>_<問題>.log、RSS は .gemm)
cd "$(dirname "$0")"
python3 gemmrss.py rail01 4 600 p5_rail01 ENOMOTO_MIP_CUTSEL_PROBE_FIT=1
python3 gemmrss.py sing44 4 600 p6_sing44 ENOMOTO_MIP_ROOTLP3=e
python3 gemmrss.py sorrell3 4 600 p7_sorrell3 ENOMOTO_MIP_ROOTCAP=k
