# 5 問の 600 秒 (ログなし、mitt600 と同じ形) を既定と試作で回す (pool_bound2、2026-11-04)。問題ごとに設定の順を入れ替える
cd "$(dirname "$0")"
i=0
for p in supportcase18 sct2 fiball neos-3627168-kasai mushroom-best; do
  if [ $((i % 2)) -eq 0 ]; then order="def h gik4"; else order="gik4 h def"; fi
  for c in $order; do
    case $c in
      def) python run.py l600_def $p 600 ;;
      h) ENOMOTO_MIP_POOL2_TAKE=h python run.py l600_h $p 600 ;;
      gik4) ENOMOTO_MIP_POOL2_TAKE=g,i,k4 python run.py l600_gik4 $p 600 ;;
    esac
  done
  i=$((i + 1))
done
