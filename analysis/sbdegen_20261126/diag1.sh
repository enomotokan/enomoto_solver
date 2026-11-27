# 診断 1: 3 問で、既定と強分枝の予算を HiGHS 並み (ENOMOTO_MIP_SB_BUDGET_HIGHS=1) を比べる。1 スレッド・種 1・120 秒
cd "$(dirname "$0")/../.."
for p in dano3_5 mcsched swath1; do
  for e in "" "ENOMOTO_MIP_SB_BUDGET_HIGHS=1"; do
    echo "== $p [$e]"
    env $e ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 timeout 300 python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit 120 --threads 1 2>&1 \
      | grep -E "^MIP: branching:|^\{" | cut -c1-300
  done
done
