# 0.2.0 に向けた誤答の確かめ (2026-11-27)

既定 = 0.1.10 + 次の変更 (既定化したものはどれも `=0` などで以前の形に戻せる。詳しくは docs/improvement_history.md の 11/21〜11/27 の節):

- **Python の入力 API (互換性のない変更)**: 変数は `M.add_variable(型, 下限, 上限, name=None)` で作る。`Variable(...)` の直接の呼び出しと、
  暗黙の「現在の Model」(`Model._stack`、`with M:`) を廃止した。`quicksum` を足し、`Function` の `+=`・`-=` をその場で足し込む形にした (70e0b1458)。
- 1 スレッドの再現: TIMEDEP=b・DET1T=onil
- カット: CUT_SMALL_FLOOR・FINISH_CANCEL・INHPOOL、CMIRGEN=u、GEN_TOP3、ROOTPAR_CUT、PATHH_SLACK
- 根の LP と手間: CUTSEL_PROBE_FIT・ROOTLP3=e・ROOTCAP=k、ipmxm (内点法 + クロスオーバーの直し)、RESTART_LPFAIL_NOCUTS
- 前処理・許容誤差: SUBMIP_FEASTOL、FZ2_FIX、LPPROBE_FIX=d
- 衝突・証明: CONFLICT_PROOF_EXPLAIN (pe)、PROOFCOST=pa
- 複数スレッド: PAR4=fmr、PARIDLE=h、PARSB
- 研磨: POLISH_FINAL (返す前の 1 回だけ、MIP の許容誤差 W_mip で最適を判定)
- 解探し: ALNS (alnsc の形、1b)
- 単体法・LU: SIMPLEX_RETRY_ZMAX、LU_REFAC_BAL=k
- そのほか: OVERRUN3=ild、UNIFY_FIX (x・z・p)、列の少ないオービトープの静的な扱い

先端 e075cbf86 (src・python は 775f61d19 と同じ) を `cargo build --release` して入れ、`bash analysis/release_20261127/fuzz.sh` で
環境変数なし (既定) で 0.1.10 と同じ fuzz 7 本と cargo test を回した。その前に check_default (6 問の道筋) も回した。出力は run.out、ログは `fuzz_logs/` (コミットしない)。

## 結果

| 命令 (既定) | 件数 | bad |
|---|---|---|
| check_default (air03・neos8・exp-1-500-5-5・qnet1・mik-250-20-75-4・markshare_4_0) | 6 | 全部一致 (11/27 の基準) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0` | 400 | 0 (strict 9) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300` | 300 | 0 |
| `python analysis/objint_20261112/fuzz_objint.py 1000 0` | 1000 | 0 (optimal 967・infeasible 33) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0` | 1000 | 0 (mismatches 0) |
| `ENOMOTO_MIP_REL_GAP=1e-9 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000` | 400 | 0 (strict 12) |
| `python analysis/objint_20261112/fuzz_objint.py 1000 5000` | 1000 | 0 (optimal 971・infeasible 29) |
| `python analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000` | 1000 | 0 (mismatches 0) |
| `cargo test --release --lib` (既定) | 426 通過・0 失敗・19 無視 | — |
| `pytest tests` (Python の API) | 69 通過 | — |

- **誤答 0、状態の食い違い 0** (7 本とも rc=0)。strict の数 (値の一致だけの以前の判定) は 0.1.10 とほぼ同じ (許容誤差の内の差)。
- 解けた数 (600 秒、Mittelmann 186 問、種 1): 11/18 の版 → 11/25 の版 (c8e468e25) で 1 スレッド 69 → 78、4 スレッド 88 → 98。
  それより後の既定化 (ALNS・LU_REFAC_BAL ほか) は、それぞれの P の対の比べで確かめた。
