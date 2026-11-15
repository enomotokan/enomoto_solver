# long600.sh の続き (途中で止まった後の残り。sct2 の def と 3 問) (pool_bound2、2026-11-04)
cd /home/user/enomoto_solver/analysis/pool_bound2_20261104
python run.py l600_def sct2 600
i=0
for p in fiball neos-3627168-kasai mushroom-best; do
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
