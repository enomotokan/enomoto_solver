#!/bin/bash
# 改良 (11-08) の計測の順 その 2: (chain2 の後、wheel を作り直してから) traininstance6 の分け方の続き (HNFBR・HNFPRE=1 のカットなし) → 置き換えの後の前処理のかけ直しのふるい
cd "$(dirname "$0")"
while pgrep -f chain2.sh >/dev/null; do sleep 20; done
python run.py dec600.jsonl 600 br,brnc,hnc 1,2 traininstance6
python run.py screen60c.jsonl 60 subre,br 1 @hits
