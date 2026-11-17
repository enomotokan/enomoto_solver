# 0.1.9 に向けた誤答の確かめ (既定 = cand1119 + カットの共有 + 空の行の許容の直し、2026-11-17)

コミット 713405e15 を取り出し (1005b567d カットの共有の既定化・5ffdfb6da 空の行の許容の直し・96163116e CLQ_LAZY の直しを含む)、
`pip install -e .` (maturin、リリース) で入れて回した (highspy 1.15.1)。`bash analysis/release019_20261117/fuzz.sh` で、環境変数なし (既定) で
0.1.8 (analysis/cand1118_default_20261116/fuzz.md) と同じ fuzz 7 本と cargo test を回した。ログは `fuzz_logs/` (コミットしない)。

## 結果

| 命令 (既定) | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 13) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000` | 400 | 0 (strict 14) |
| `python analysis/objint_20261112/fuzz_objint.py 1000 5000` | 1000 | 0 (optimal 971・infeasible 29) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000` | 1000 | 0 (mismatches 0) |
| `cargo test --release --lib` (既定) | 419 通過・0 失敗・19 無視 | — |

- **誤答 0、状態の食い違い 0** (7 本とも rc=0)。strict の数は 0.1.8 と同じ (許容誤差の内の差)。
- 時間: fuzz_tiny 3.7 秒、fuzz_highs 5.3 秒、fuzz_objint 75.1 秒 (種 5000 側 72.5 秒)。
- このほか、0.1.8 からの既定の変更はそれぞれ P・M の対の比べで誤答 0 (cand1119: analysis/cand1119_20261117/、カットの共有: analysis/sharecuts_20261117/)、
  空の行の許容の直しは mip_fuzz_vs_highs 3000 問 bad 0 (analysis/wrongans_20261117/)。
