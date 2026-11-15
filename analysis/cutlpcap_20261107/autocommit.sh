#!/bin/bash
# 計測の途中データを 15 分ごとにコミット・push する (コンテナの再起動に備える)
cd "$(dirname "$0")"
while true; do sleep 900; ./push.sh >> autocommit.log 2>&1; done
