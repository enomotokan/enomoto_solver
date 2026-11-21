# 候補なしのやり直しを入れた配る a (a2) を 203 問で測り直す (pkg45、ほかの設定は chain.sh と同じ)
export PYTHONPATH=/tmp/claude-0/-home-user-enomoto-solver/73d0c415-6aab-5ce0-b6a6-d7bc7782e4ae/scratchpad/pkg45
cd "$(dirname "$0")"
SA_OUT=a2 SA_ENV=ENOMOTO_MIP_SEPALLOC=a python3 run_root.py 120 4 > run_a2.out 2>&1
echo done > chain2.done
