#!/bin/bash
# 計測の待ち行列 2: 58 問 (4 スレッド・60 秒) を既定と cs で 2 回 (rep 1・2)。再スタートの後の尺度を直したビルド (基準 = 最初の根のカットの前の LP)
cd $(dirname $0)
python run58.py 1 def cs
python run58.py 2 def cs
echo q2 done
