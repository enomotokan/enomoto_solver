# 続きの計測 (止まったら同じ命令で続きから: run.py は済んだ回を飛ばす)
cd /home/user/enomoto_solver
python analysis/hnfpre_20261107/run.py analysis/hnfpre_20261107/lu58.jsonl 60 def,luold 1,2 @p58 >> /tmp/lu58.log 2>&1
python analysis/hnfpre_20261107/run.py analysis/hnfpre_20261107/hnfb600.jsonl 600 def,hnfb 1,2 maritime-jg3d9 atlanta-ip sing326 >> /tmp/hnfb600.log 2>&1
echo CHAIN2_DONE >> /tmp/hnfb600.log
