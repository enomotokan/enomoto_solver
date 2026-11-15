# 正しさの検査の一式 (結果は fuzz_*.txt)
cd /home/user/enomoto_solver/analysis/symdyn_20261110
D="ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_NO_CUTS=1 ENOMOTO_MIP_NO_RESTART=1"
for sc in 1 2 3; do
  env ENOMOTO_MIP_SYMREFL=1 $D python fuzz_symdyn.py 200 0 $sc > fuzz_refl_hard_s$sc.txt 2>&1
  env ENOMOTO_MIP_SYMREFL=1 python fuzz_symdyn.py 200 1000 $sc > fuzz_refl_s$sc.txt 2>&1
  env ENOMOTO_MIP_SYMREFL=1 ENOMOTO_MIP_THREADS=4 $D python fuzz_symdyn.py 100 2000 $sc > fuzz_refl_hard_4t_s$sc.txt 2>&1
done
env ENOMOTO_MIP_SYMDYN=1 python ../../scripts/mip_fuzz_symmetric.py 150 0 1 > fuzz_symm_s1.txt 2>&1
env ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_NO_ORBITOPE=1 python ../../scripts/mip_fuzz_symmetric.py 150 0 1 > fuzz_symm_noorb_s1.txt 2>&1
env ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_NO_ORBITOPE=1 python ../../scripts/mip_fuzz_symmetric.py 60 0 2 > fuzz_symm_noorb_s2.txt 2>&1
env ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_NO_ORBITOPE=1 python ../../scripts/mip_fuzz_symmetric.py 40 0 3 > fuzz_symm_noorb_s3.txt 2>&1
env ENOMOTO_MIP_SYMDYN=1 python ../sym_of_20261028/fuzz_of.py 100 0 > fuzz_of_s1.txt 2>&1
env ENOMOTO_MIP_SYMDYN=1 ENOMOTO_MIP_NO_CUTS=1 python ../sym_of_20261028/fuzz_of.py 100 0 2 > fuzz_of_nocut_s2.txt 2>&1
echo ALLDONE > fuzz_done.txt
