# 副作用の確認: 5 問 x (既定, full) x 2 回、4 スレッド・60 秒。結果は side_results.txt に 1 行ずつ
cd /home/user/enomoto_solver/work_tree
F="ENOMOTO_MIP_LP_INT_BOUNDS=1 ENOMOTO_MIP_PROOF_SIDE_REDUCE=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=1"
for r in 1 2; do
 for prob in neos-860300 eil33-2 binkar10_1 10teams mas76; do
  for cfg in base full; do
   if [ $cfg = base ]; then E="ENOMOTO_X=1"; else E="$F"; fi
   env ENOMOTO_MIP_THREADS=4 $E timeout 200 python run_eno.py ../.miplib_cache/$prob.mps 60 4 > side_${prob}_${cfg}_$r.out 2>/dev/null
   echo "$prob $cfg $r $(grep -E '^ +[0-9.]+s ' side_${prob}_${cfg}_$r.out | tail -1 | awk '{print $2, $4, $5}') $(tail -1 side_${prob}_${cfg}_$r.out)" >> side_results.txt
  done
 done
done
