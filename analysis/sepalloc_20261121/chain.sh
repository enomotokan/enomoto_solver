# 根だけの 203 問の比べ (1 スレッド・決定的な設定・120 秒・4 本並べる): 既定 def、配る a、止める s、両方 as
export PYTHONPATH=/tmp/claude-0/-home-user-enomoto-solver/73d0c415-6aab-5ce0-b6a6-d7bc7782e4ae/scratchpad/pkg44
cd "$(dirname "$0")"
SA_OUT=def python3 run_root.py 120 4 > run_def.out 2>&1
for t in a s as; do SA_OUT=$t SA_ENV=ENOMOTO_MIP_SEPALLOC=$t python3 run_root.py 120 4 > run_$t.out 2>&1; done
echo done > chain.done
