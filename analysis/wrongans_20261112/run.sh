#!/bin/bash
# 誤答 3 件の再現の確かめ (2026-11-12)。rep.py (analysis/wrongans_20261105 の写し) で解き、返した解を元の問題で確かめて *.jsonl に足す。
# 1) traininstance6 既定 4 スレッド・600 秒 種 1〜2 (続けて 3〜4)
# 2) その後、traininstance6 既定 1 スレッド・600 秒 種 1 と並べて、binkar10_1 (ROBUST_STALL=rw) と mc11 (SUBMIP_CUTS 系) を 4 スレッド・60 秒で 2 種ずつ
cd $(dirname $0)
export REP_LOG=1
python rep.py traininstance6 def 1 3 t6_def4.jsonl 600 4 1
( python rep.py traininstance6 def 1 2 t6_def1.jsonl 600 1 1 ) &
python rep.py binkar10_1 stallrw 1 3 bk_stallrw.jsonl 60 4 1
python rep.py mc11 "ENOMOTO_MIP_SUBMIP_CUTS=1" 1 3 mc11_subcuts.jsonl 60 4 1
python rep.py mc11 "ENOMOTO_MIP_SUBMIP_CUTS=1;ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25;ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10" 1 3 mc11_subr25.jsonl 60 4 1
wait
python rep.py traininstance6 def 3 5 t6_def4.jsonl 600 4 1
