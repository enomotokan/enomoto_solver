# mushroom-best の再スタートの模型の根 (HiGHS と ENOMOTO の比べ)

1. 模型の書き出し (20 秒で根の再スタート 1 まで進む):
   `env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_DEBUG_DUMP_RESTART=/tmp/mdump/m python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/mushroom-best.mps --time-limit 20 --threads 4`
   → `/tmp/mdump/m_r1.mps` (9197 行 × 8468 列、持ち越したカットを行として含む。4.5 MB なので置いていない)。
2. HiGHS の根だけ: `python -I hroot.py /tmp/mdump/m_r1.mps 1 0.0553338` (ノード上限 1、`objective_bound` = 最適値) → `highs_root_m_r1.out`
   (根 0.015842 → 0.022547 (カット 11128 本) → 0.023095)。
3. ENOMOTO の根だけ: `env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_NODE_LIMIT=1 ENOMOTO_MIP_CUTOFF=0.0553338 ENOMOTO_MIP_DEBUG_SEP=1 python scripts/run_miplib_benchmark.py --worker-ours /tmp/mdump/m_r1.mps --time-limit 60 --threads 1`
   → `enomoto_root_m_r1.{out,err}` (根 0.015850 のまま。`SEP rows`・`SEP path` などの行が分離ルーチンごとの候補の数)。
4. `insp.py` は模型の列・行の形を数える (`python -I insp.py <mps>`)。
