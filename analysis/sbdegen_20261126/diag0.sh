# 診断 0: 強分枝の判断のうち双対退化で省いた数 (MIP: branching の行)・強分枝の LP の数と反復、1 スレッド・種 1・120 秒
cd "$(dirname "$0")/../.."
for p in dano3_5 mcsched swath1 30n20b8; do
  echo "== $p"
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 timeout 300 python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit 120 --threads 1 2>&1 \
    | grep -E "^MIP: branching:|^MIP: strong branching|reliab|^\{" | cut -c1-300
done
