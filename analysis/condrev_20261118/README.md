# CONDREV: 一部で大きく悪化した試作 3 件を条件付きで入れ直す (2026-11-18)

依頼は `BRIEF.md` (利用者の承認済み)。既定は変えない。試作は環境変数で入れる (`src/mip/condrev.rs`)。

## 1. 前提の確かめ (今の既定、db187e1a3)

- sbt (`ENOMOTO_MIP_SB_TIGHTEN`・`_INHERIT`・`_ANCESTOR`・`ENOMOTO_MIP_SB_POLY_RC`): 実装は残っている (solver.rs)。
- SUBFIX: 既定は今も `rtcqf` (heur_subfix.rs)。`ENOMOTO_MIP_SUBFIX=0` は SUBFIX のダイブをやめ、既定の RENS・RINS に戻す
  (サブ MIP そのものを止めるのではない)。
- ROOTSTOP: 実装は残っている (rootstop.rs)。既定は `b` になっているので、`rb` は既定に `r` を足す形。

### 一般整数の列の数 (前処理の前、58 問)

- 一般整数の列 (整数で上下限の幅が 1 を越える列) は、11 本以上の 16 問 (summ6.py の GEN と同じ 16 問) と、4 本以下の 42 問にはっきり分かれる。
  境目の 5〜10 本の問題はない。neos-911970 は 0 本 (整数列 840 本はすべて 0-1)。
- 大きい順: neos-3083819-nubu 8644、roll3000 492、mzzv42z 235、neos-1456979 180、mik-250-20-75-4 175、gt2 164、qnet1 129、timtab1 94、
  30n20b8 62、gen-ip002 41、nu25-pr12 36、gen-ip054 30、blend2 25、noswot 25、rout 15、flugpl 11。その下は neos8 4、hypothyroid-k1 1、ほかは 0。
- よって sbt の条件は普通の if (`n_genint >= 10`) にした。10teams (一般整数 0 本) の良化は条件付きでは失う。

### SUBFIX の害と得 (種 1〜4、60 秒、今の既定)

- neos-960392 (整数列 59376 本): 既定は SUBFIX の根の RENS (固定率 0.62、自由な整数列 621/1625) が 0.05 秒で最適解を出し、ノード 0 で 1.3〜1.5 秒。
  `SUBFIX=0` は種 1 で 1.4 秒 (ノード 0) だが、種 2〜4 で 4.5・2.6・3.4 秒 (ノード 612・145・213)。記録 (cand1113 loo.md) の害は今も出る。
- neos-827175 (整数列 21350 本): 種 1 で既定 5.9 秒・`SUBFIX=0` 5.6 秒 (どちらも根で決着)。
- 害の neos-960392 が整数列 5.9 万本、記録の 421 秒の neos-873061 が 8.8 万本なので、**整数列の数では分けられない** (`n_int > 2 万` で切ると neos-960392 を壊す)。
  そこで SUBFIX は止めず、外側の根の SUBFIX のサブ MIP の時間に上限を付ける形 (`sfcap`) を主にした。整数列の数で切る形
  (`ENOMOTO_MIP_SUBFIX_OFF_NINT`) も実装はしたが、計測には入れていない。

## 2. 実装 (`src/mip/condrev.rs`、既定では何もしない)

- 外側の求解の始め (`solve_problem` の外側、前処理の前の問題) に 1 回決め、全スレッドで共通の値を使う。
- `ENOMOTO_MIP_SBT_GENINT_MIN=k`: 一般整数の列が k 本未満なら、sbt の 4 つの環境変数を付けていても切る。
- `ENOMOTO_MIP_SUBFIX_OFF_NINT=k`: 整数列が k 本を越えるなら `ENOMOTO_MIP_SUBFIX=0` とみなす。
- `ENOMOTO_T_MIP_SUBFIX_CAP=f`: 外側の根の SUBFIX のサブ MIP (ダイブの後) の時間の上限を、時間の上限の f 倍 (最小 `ENOMOTO_T_MIP_SUBFIX_CAP_MIN` 0.5 秒) にする。
  既存の `PRIMAL_TIME_CAP` (試作 PRIMAL の `x` の仕組み) に載せた。60 秒なら 3 秒、600 秒なら 30 秒。
- ログ (`ENOMOTO_MIP_LOG=1`): `MIP: CONDREV: integer columns …, general integer columns …; sbt off/as set, SUBFIX off/as set`。

## 3. 短い計測 S (`planS.py` → `shards_S.json`、`run.py S k`)

設定は `cfg.py`: def・sbt・sbtc (`SBT_GENINT_MIN=10`)・sf0 (`SUBFIX=0`)・sfcap (`SUBFIX_CAP=0.05`)・rb (`ROOTSTOP=rb`・`RATE=0.02`)。

- sbt: 一般整数 16 問 + neos-911970・10teams × {def, sbt, sbtc}
- SUBFIX (60 秒): neos-960392・10teams・neos-827175・noswot・sp150x300d × {def, sf0, sfcap}
- SUBFIX (600 秒、種 1・2): neos-873061・neos859080・fiball・supportcase33・mzzv42z・pg5_34 × {def, sf0, sfcap}
- rb: 10teams・noswot・mc11・neos5・neos-911970・mik-250-20-75-4・swath1・fiber・p200x1188c・timtab1・eil33-2・dcmulti × {def, rb}
- 60 秒の問題は種 1〜4。問題ごとに設定の和集合を同じ (問題, 種) で続けて解く。122 組 374 回、6 分担で 1 分担 約 1 時間 (計 5.8 時間)。

`cargo test --release --lib` (RUSTFLAGS に `-L/usr/local/lib -lpython3.13`): 412 通過、0 失敗。

(結果は後で書く)
