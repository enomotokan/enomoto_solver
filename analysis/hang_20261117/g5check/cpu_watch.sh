# 使い方: cpu_watch.sh pid  — SOLVE_DONE の後、1 秒ごとにスレッドごとの CPU 時間 (utime+stime, tick) を出す
pid=$1
for i in $(seq 1 11); do
  echo "--- $(date +%s.%N)"
  for t in /proc/$pid/task/*; do n=$(cat $t/comm 2>/dev/null); s=$(awk '{print $14+$15}' $t/stat 2>/dev/null); echo "$n $s"; done | grep enomoto | sort | awk '{print}'
  sleep 1
done
