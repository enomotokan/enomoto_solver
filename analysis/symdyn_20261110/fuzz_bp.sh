cd /home/user/enomoto_solver/analysis/symdyn_20261110
H="ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_NO_CUTS=1 ENOMOTO_MIP_NO_RESTART=1 FUZZ_KINDS=binpack,gbinpack"
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SYMDYN=1 $H python fuzz_symdyn.py 150 0 3 > fuzz_bp_hard_s3_log.txt 2>&1
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_THREADS=4 $H python fuzz_symdyn.py 100 1000 3 > fuzz_bp_hard_4t_s3_log.txt 2>&1
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_NO_CUTS=1 FUZZ_KINDS=binpack,gbinpack python fuzz_symdyn.py 100 2000 3 > fuzz_bp_orb_s3_log.txt 2>&1
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SYMREFL=1 $H FUZZ_KINDS=binpack,gbinpack,maxcut,bisect,cutcover,intrefl python fuzz_symdyn.py 100 3000 4 > fuzz_mix_hard_s4_log.txt 2>&1
echo DONE3 >> fuzz_done.txt
