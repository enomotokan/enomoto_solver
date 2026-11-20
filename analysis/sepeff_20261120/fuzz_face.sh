# FACECUT の誤答の確かめ (HiGHS と最適値の突き合わせ)。対称の割り当て問題の倍率 2・3 で根のカットが停滞し、FACECUT がカットを足す (20 問中 4・1 問)
export PYTHONPATH=/tmp/claude-0/-home-user-enomoto-solver/73d0c415-6aab-5ce0-b6a6-d7bc7782e4ae/scratchpad/pkg41 ENOMOTO_MIP_FACECUT=1 ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_MIP_THREADS=1
while pgrep -f mip_fuzz_vs_highs > /dev/null || pgrep -f "mip_fuzz_symmetric.py 300" > /dev/null; do sleep 30; done
nice python3 /home/user/enomoto_solver/scripts/mip_fuzz_symmetric.py 1500 0 2 2>&1 | grep -E "^bad|^seed " > fuzz_sym2.txt
nice python3 /home/user/enomoto_solver/scripts/mip_fuzz_symmetric.py 500 0 3 2>&1 | grep -E "^bad|^seed " > fuzz_sym3.txt
echo done > fuzz_face.done
