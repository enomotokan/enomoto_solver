#!/bin/bash
# 58 問 (sgl) の後: 今のビルド (別のセッションのコミットの上へ rebase した後、par3sg・par3sgl と同じビルド) で beasleyC3 の既定を種 1〜8 回し直す (名前 defnew)。結果は batch7.txt
cd "$(dirname "$0")"
until grep -q ALLDONE /tmp/b58sgl.log 2>/dev/null; do sleep 15; done
for s in 1 2 3 4 5 6 7 8; do
  ./run1.sh beasleyC3 beasleyC3_defnew_s$s 4 $s 300 >> batch7.txt
done
echo ALLDONE >> batch7.txt
