#!/bin/bash
# 600 秒の比べと 58 問 (60 秒、2 回) を順に回す (ソルバーは同時に 1 つ)
cd "$(dirname "$0")"
python run.py def 600 1 neos-3216931-puriri
python run.py h1mv 600 1 neos-3216931-puriri
python run.py h1 600 1 neos-3216931-puriri
for p in satellites2-60-fs neos-873061 mzzv11 ns1830653; do
  python run.py def 600 1 $p
  python run.py h1mv 600 1 $p
done
python bench58.py def,h1mv 1
python bench58.py def,h1mv 2
