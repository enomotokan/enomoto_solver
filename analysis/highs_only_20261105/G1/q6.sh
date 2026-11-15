cd /home/user/enomoto_solver/analysis/highs_only_20261105/G1
while pgrep -f q5.sh > /dev/null; do sleep 10; done
python run58.py def sc re > run58.log 2>&1
echo ALLDONE >> run58.log
