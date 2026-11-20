# 使う価値のなくなった試作の片付け 第 1 弾 (2026-11-18、利用者の承認済み)

不採用の根拠が記録で確かめられた試作のうち、誤答の元 2 件と規模の大きい 10 件を消す。既定の振る舞いは変えない。

## 対象 (この順に 1 件ずつ)

| # | 試作 (環境変数) | 主なファイル | 不採用の根拠 (見当、消す前に記録で確かめる) |
|---|---|---|---|
| 1 | `ENOMOTO_MIP_PROBE_EQUIV2_RESTART` | src/mip/mod.rs | 誤った最適値が出る |
| 2 | `ENOMOTO_MIP_QIU_MARGIN_DEPTH` | src/mip/mod.rs | 下限を破る |
| 3 | `ENOMOTO_MIP_FPR` 一式 (`T_MIP_FPR_*`・`MIP_DEBUG_FPR`) | src/mip/fpr.rs (約 1500 行) | +1.6%〜+9.6% |
| 4 | `ENOMOTO_MIP_MODKVB` 一式 (`_CAPDROP`・`_CG`・`_DUMP`・`_IV`・`_K2`・`_LOG`・`_NOCUTROWS` …) | src/mip/modkvb.rs、separation.rs、snfmodk.rs・hnfcut.rs の参照 | 解けた数 84 → 79 |
| 5 | `ENOMOTO_MIP_SEPLEARN` 一式 (`T_MIP_SEPLEARN_*`) | src/mip/seplearn.rs、separation.rs、solver.rs | 解けた数 85 → 81 |
| 6 | `ENOMOTO_MIP_REDSPLIT` 一式 | src/mip/redsplit.rs、separation.rs、cuts.rs | neos5 が遅い |
| 7 | `ENOMOTO_MIP_LB2`・`ENOMOTO_MIP_PROX2`・`ENOMOTO_MIP_HSUB_STALL` 一式 | src/mip/heur_lns2.rs、solver.rs | 解けた数が減る |
| 8 | `ENOMOTO_MIP_CUTSEL_ENS` 一式 | src/mip/cutsel_ens.rs、separation.rs | +9.2% |
| 9 | `ENOMOTO_MIP_GCDFORCE` 一式 | src/mip/gcdforce.rs、mod.rs | 当たる行が 0 本 |
| 10 | `ENOMOTO_MIP_OBJ_SUPPORT` 一式 | src/mip/solver.rs、heuristics.rs | 不採用 |
| 11 | `ENOMOTO_MIP_BRCUT` 一式 | src/mip/brcut.rs、separation.rs、pseudocost.rs | 効果なし |
| 12 | `ENOMOTO_MIP_IMPLINT_TU` (旧版のみ。`IMPLINT_TU2` は残す) | src/mip/implint_tu.rs ほか | +164% |

棚卸しの元資料は親のスクラッチパッドにあり、ここには写していない。根拠は analysis/ と docs/improvement_history.md を
環境変数の名前で grep して確かめる。

## 注意

- 1 件でも根拠が記録で確かめられない、または既定の経路が依存している (試作の関数・構造体を既定の経路が使っている) ときは、
  消さずに飛ばし、理由を README に書いて親に知らせる。
- 共有の部品 (例えば MODKVB と snfmodk・hnfcut が共用する関数) は、他の試作や既定がまだ使っていれば残す。
- 計測中の試作 (VUBAGG・ORBPP・tab (TAB_HIGHS_SIG)・clqprb・sparsify・cutlen・cutcoef・objrow・LAZYFRAC・G5INH・G5SEQ・
  SUBWAIT・SYMDYN・S3GAP) には触らない。
- 他のセッションが同じファイル (separation.rs・solver.rs・mod.rs) を触っているので、1 件ごとに
  fetch+merge → 削除 → 確かめ → コミット → push とし、衝突を小さくする。rebase・force-push はしない。
- 消した試作を参照している analysis/ の台本 (cfg.py など) は直さない (過去の記録として残す)。
  docs/ にその試作の使い方の説明があれば「削除済み (日付)」と書き足す。

## 1 件ごとの確かめ

1. `cargo build --release` で警告が増えていない (未使用になった関数・import も消す)。
2. `cargo test --release` が全件通る。
3. 既定の振る舞いが変わっていないこと: 1 スレッド・決まった種で、P の数問 (例: p0201・mas76・pk1・neos5・rout・10teams・
   noswot・gen-ip002、各 60 秒以内に解けるもの) の (状態・目的値・ノード数) が消す前のビルドと完全に一致する。
   消す前のビルドの値を最初に 1 回取っておき、毎回それと比べる (台本 `ident.py` をこのディレクトリに作る)。
4. 4 件ごとと最後に、fuzz (release019 と同じ 7 本: fuzz_tiny・mip_fuzz_vs_highs・fuzz_objint・fuzz_slack ほか) で bad 0。

## 記録

- `README.md` に 1 件ごとの行 (消したコミット・減った行数・確かめの結果・飛ばした理由) を書く。
- 最後に docs/improvement_history.md に「## 2026-11-18: 不採用の試作 12 件を削除 (片付け 第 1 弾)」の節を足す。
- 終わったら親に、消した件数・減った行数・飛ばしたもの・確かめの結果を短く報告する。2 回目 (残りの候補 A 87 件) の要否は親が決める。

## 第 2 弾 (2026-11-18、利用者の承認済み)

対象は棚卸しで「不採用の根拠が記録で確かめられた (候補 A)」とした残りの 84 件 (`cands2.txt`、環境変数と src での所在)。
第 1 弾と同じ手順・同じ確かめで、小さいものは同じファイルの数件をまとめて 1 コミットにしてよい (1 コミット 5 件程度まで)。

- 1 件ずつ、消す前に analysis/・docs/ を環境変数の名前で grep して「不採用」の根拠を確かめる。確かめられない・計測中・既定の経路が依存する・
  他の試作が共用する部品は飛ばし、理由を README に書く。
- 次は今日計測中・判断待ちなので触らない: SYMDYN 系 (LexRed の SYMREFL・SYM_LEX_CANON を含む。LexRed の直しの記録と照らし、
  直した LexRed が使う部品は残す)、ROOTLP3 (群 5 の G5CANCEL の経路)、IMPLINT_TU2、TAB_HIGHS_SIG、CLQPRB、SPARSIFY_LATE、DOMCOL 系、
  SUBWAIT、VUBAGG・ORBPP、G5*、S3GAP。
- 4 件目ごと・最後に fuzz 7 本、毎回 cargo test・警告・1 スレッドの 12 問の一致 (第 1 弾の ident.py)。
- 記録は README に第 2 弾の節、最後に docs/improvement_history.md に節を足す。終わったら親に件数・行数・飛ばしたものを報告。
