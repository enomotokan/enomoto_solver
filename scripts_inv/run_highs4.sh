#!/bin/bash
# 使い方: run_highs4.sh <名前> <問題> <時間上限>   HiGHS を 4 スレッドで回して runs4/<名前>.log に書く
name=$1; prob=$2; T=$3
python - > /home/user/enomoto_solver/runs4/$name.log 2>&1 <<PY
import highspy
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("time_limit", float($T)); h.setOptionValue("output_flag", True)
h.readModel("/home/user/enomoto_solver/.miplib_cache/$prob.mps"); h.run()
i = h.getInfo(); print("RESULT", h.modelStatusToString(h.getModelStatus()), i.objective_function_value, i.mip_dual_bound, i.mip_node_count)
PY
tail -1 /home/user/enomoto_solver/runs4/$name.log
