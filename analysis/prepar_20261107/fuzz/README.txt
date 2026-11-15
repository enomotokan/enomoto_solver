誤答の検査 (最終の wheel、コミット fc1a60db、4 スレッド、ENOMOTO_MIP_REL_GAP=1e-9)。食い違いの行と最後の bad の行だけ残した。
tiny_ple.txt / tiny_def.txt: benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 (ple / 既定)
vs_highs_ple.txt: scripts/mip_fuzz_vs_highs.py 300 (ple)
probe_ple.txt / probe_def.txt: analysis/prepar_20261107/fuzz_probe.py 300 0 (ple / 既定)
probe_ple_small.txt: fuzz_probe.py 300 1000、ple で塊を最小にした形 (ENOMOTO_T_MIP_PREPAR_BSZ=2 _BSZ_MAX=2 _EQ_BSZ=2 _LP_BSZ_MAX=1。窓の作り直し・まとめを多く通す)
