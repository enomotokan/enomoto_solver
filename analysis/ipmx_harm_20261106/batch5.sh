#!/bin/bash
# 対象の 8 問: 小さい 3 問は 3 回、大きい 5 問は 2 回 (新しいビルド、_v2)
cd "$(dirname "$0")"
RUN_TAG=_v2 python3 run.py def,ipmx,ipmxm mc11,noswot,qnet1 3
RUN_TAG=_v2 python3 run.py def,ipmx,ipmxm dano3_3,chromaticindex512-7,physiciansched6-2,neos-933966,satellites2-40 2
