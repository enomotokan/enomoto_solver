# MIPLIB の MPS (計測用の写し)

miplib.zib.de が使えないときのための、計測に使う 55 問 (MIPLIB 40 問 + HiGHS だけが解く 16 問の重なり分) の MPS の写し。
使い方: `git fetch origin inv/mps-cache-20261021 && mkdir -p .miplib_cache && for f in $(git ls-tree --name-only origin/inv/mps-cache-20261021 mps/); do git show origin/inv/mps-cache-20261021:$f | gunzip > .miplib_cache/$(basename $f .gz); done`
