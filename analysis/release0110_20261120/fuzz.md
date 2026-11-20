# 0.1.10 に向けた誤答の確かめ (2026-11-20)

既定 = 0.1.9 + 次の既定化 (どれも `=0` などで以前の形に戻せる。詳しくは docs/improvement_history.md):

- st5 (d35ca2d7d)、SYM_PPLEX (2b0230c56)、cwrh0 (c77c44240)
- vbce: `ENOMOTO_MIP_VBCLQ=e` (decomp2 の直しを含む、ee58b9b89)
- ncnr: `ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN`・`ENOMOTO_MIP_FEAS_NORETRY`、rhb: `ENOMOTO_MIP_RHBUDGET` (4aa32b32a)
- cb: `ENOMOTO_MIP_RESTART_DROP_FIXED`・`ENOMOTO_MIP_PRESOLVE_STOP_FINE` (00a80b6d4)
- 計測の台本のモデル読み込みを当方の MPS 読み込み (`Model.readModel`、src/mps.rs) に切り替え (a7e9d28e8・ef9792fac)

コミット 16e1b78a1 を `pip install -e .` で入れ、`bash analysis/release0110_20261120/fuzz.sh` で環境変数なし (既定) で 0.1.9 と同じ fuzz 7 本と cargo test を回した。
ログは `fuzz_logs/` (コミットしない)。

## 結果

| 命令 (既定) | 件数 | bad |
|---|---|---|
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 12) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000` | 400 | 0 (strict 13) |
| `python analysis/objint_20261112/fuzz_objint.py 1000 5000` | 1000 | 0 (optimal 971・infeasible 29) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000` | 1000 | 0 (mismatches 0) |
| `cargo test --release --lib` (既定) | 419 通過・0 失敗・19 無視 | — |

- **誤答 0、状態の食い違い 0** (7 本とも rc=0)。strict の数は 0.1.9 とほぼ同じ (許容誤差の内の差)。
- このほか、各既定化はそれぞれ P・M の対の比べで食い違い 0 (analysis/scipgate_20261120・rootlp_20261120・objtarget_20261119 ほか)。
