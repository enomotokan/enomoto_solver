#!/bin/bash
# ACFIX に LP での確かめを入れた後の回 (all2 = all と同じ環境変数、新しいビルド) と 58 問の ho4b の回し直し
cd "$(dirname "$0")"
export G4_TMP=/tmp/claude-0
python run.py ours all2 600 satellites2-60-fs
python bench58.py def,ho4b 0 > /tmp/claude-0/bench58b.out 2>&1
