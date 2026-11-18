# CONDREV: 一部で大きく悪化した試作 3 件を条件付きで入れ直す (2026-11-18)

**閉じた (2026-11-18、親の判断)**: 3 件とも広い計測に進めない (rb の 58 問の測り直しも見送り)。理由は §6〜7。

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

## 4. 誤答の確かめ (`fuzz.sh`)

analysis/release019_20261117/fuzz.sh と同じ 7 本を、3 件の試作を全部入れて回した (sbt + `SBT_GENINT_MIN=1`・`SUBFIX_CAP=0.05`・`ROOTSTOP=rb`・`RATE=0.02`。
`SBT_GENINT_MIN=1` は、一般整数の列のある問題で sbt を入れ、ない問題で切る両方の道を通すため)。ビルドは d72c9e6c7。

| 本 | 結果 |
|---|---|
| fuzz_tiny 400 0 | bad 0 of 400 |
| mip_fuzz_vs_highs 300 | bad 0 of 300 |
| fuzz_objint 1000 0 | bad 0 of 1000 |
| fuzz_slack 1000 0 | mismatches 0 |
| fuzz_tiny 400 1000 | bad 0 of 400 |
| fuzz_objint 1000 5000 | bad 0 of 1000 |
| fuzz_slack 1000 5000 | mismatches 0 |

## 5. neos-873061 で sfcap が働くか (ログつき、種 1、600 秒、d72c9e6c7)

- 今の既定では、根に入るまでが長い (根のカットの 1 ラウンド目が 334 秒、根のヒューリスティクスの終わりが 562 秒)。
  SUBFIX の根の RENS (固定率 0.60、自由な整数列 35034/87610) のサブ MIP は残り時間が少なく 0.9 秒で時間切れになり、上限 (30 秒) に届かない。
- よって記録の「SUBFIX の RENS 1 回 421 秒」(prune_20261112.md §3、d644a0e0) は今の既定では起きておらず、S の neos-873061 で sfcap と def が
  同じ結果 (目的値・下界が同じ) なのはこのため。sfcap が狙った害は今は見えない。

## 6. 短い計測 S の結果 (d72c9e6c7、6 分担 374 回、crash 0、`summS.py` → `summS.txt`)

分担 0 はこのセッション、1〜5 は親が割り振ったセッションで回した (機械は分担ごとに違う)。両方解けた回の最適値は設定の間で一致した。
比は「試作 / def」(shift 1 秒の SGM、時間切れは上限で数える、どちらかが解けた問題。区間は問題単位のブートストラップ 95%)。

| 試作 | 対 | SGM 比 [95%] | 解けた (def → 試作) | 見立て |
|---|---|---|---|---|
| sbt (無条件) | 70 | +2.7% [−3.1, +9.1] | 52 → 52 | 得が見えない |
| sbtc (`SBT_GENINT_MIN=10`) | 70 | +1.3% [−2.5, +6.1] | 52 → 50 | 得が見えない (落とした 2 回は noswot の揺れ) |
| sf0 (`SUBFIX=0`) | 32 | +11.6% [−2.0, +30.5] | 30 → 30 | 悪い。neos-960392 は 4 種とも遅い |
| sfcap (`SUBFIX_CAP=0.05`) | 32 | −1.0% [−9.1, +8.3] | 30 → 28 | 揺れの中。狙った害 (neos-873061) が今は起きない (§5) |
| rb (`ROOTSTOP=rb`・`RATE=0.02`) | 48 | −0.3% [−6.7, +7.5] | 47 → 46 | 揺れの中。eil33-2 が 4 種とも遅い |

### sbt

- 一般整数 16 問での以前の得 (−4〜−6%、cbcimpl_20261029.md 6 段目) は、今の既定では出なかった。差のある問題は揺れの大きいものばかり
  (noswot・rout・timtab1・roll3000・neos-3083819-nubu)。timtab1 は sbt で 4 種とも少し遅い (37.0・17.8・27.8・20.4 → 44.7・22.0・33.7・21.3 秒)。
  時間切れの問題では sbt・sbtc の終わりの暫定解がよい回がある (neos-1456979・gen-ip002、分担の報告)。
- 一般整数 16 問では sbt と sbtc は同じ設定なので、その差は揺れの大きさを示す。noswot は sbt 40.7・19.0・50.4・10.5 秒、sbtc 時間切れ・時間切れ・11.9・17.6 秒。
- **neos-911970 の害は揺れと区別できない**。sbtc は neos-911970 で sbt を切る (一般整数 0 本) ので def と同じ設定だが、def 20.9・19.2・10.2・16.8 秒に対して
  sbtc 21.0・18.1・14.7・32.7 秒、sbt 25.0・18.8・15.3・26.5 秒。以前の記録の「3 種とも約 1.3 倍か時間切れ」も、この揺れの幅に入る。

### SUBFIX

- sf0 は neos-960392 を 4 種とも遅くする (3.0・1.7・1.7・1.5 → 5.3・4.8・3.0・4.4 秒)。600 秒の neos859080 も 2 種とも遅い (397・300 → 515・499 秒)。
  得は supportcase33 の 1 種 (484 → 297 秒) と noswot の揺れだけ。無条件で止める形は退ける。
- sfcap は neos-960392 を壊さない (2.9・1.5・1.8・1.7 秒)。neos859080 は 112 秒・時間切れ、fiball は時間切れ・25 秒と、種ごとに大きく割れる。
  §5 のとおり、今の既定では neos-873061 の SUBFIX のサブ MIP は 0.9 秒で終わっており、上限が効く場面が見つからない。

### rb

- 速い向き: mik-250-20-75-4 (4 種とも、2.7〜4.1 → 1.6〜4.0 秒)・mc11 (4 種とも少し)・timtab1・swath1 (3/4)。
- 遅い向き: **eil33-2 が 4 種とも** (43.2・20.0・24.8・24.1 → 48.6・30.2・32.9・32.9 秒、1.12〜1.51 倍)、p200x1188c (2/4)。
  neos5・noswot は種ごとに入れ替わる (時間切れが def 1 回・rb 2 回)。

## 7. 判断

- 3 件とも、短い計測で「良い」と言える形にならなかった。BRIEF の段取り 4 (P 4 分担・M 12 分担の広い計測) には進めない、を勧める。
  - sbt: 以前の得が今の既定では再現しない。neos-911970 の害も揺れと区別できないので、条件を付ける根拠そのものが弱い。
  - SUBFIX: 無条件で止めるのは害がはっきりしている (neos-960392)。条件付きの sfcap は、狙った害が今は起きない。
  - rb: 揃った得がなく、eil33-2 が 4 種とも遅い。
- 試作の環境変数 (`ENOMOTO_MIP_SBT_GENINT_MIN`・`ENOMOTO_MIP_SUBFIX_OFF_NINT`・`ENOMOTO_T_MIP_SUBFIX_CAP`) は既定では何もしないので、src に残してある。
- 計測の不備: planS.py は (問題, 種) を鍵にしたため、mzzv42z の 60 秒の sbt の組の種 1・2 が 600 秒の SUBFIX の組に上書きされ、回していない
  (mzzv42z は 60 秒では def・sbt・sbtc とも時間切れなので、結論には効かない)。分担 0 は再起動の後の再開で、fiball の sfcap 種 1 の初めの 5 秒ほどを
  同じ回の写しと重ねて回した (その回は時間切れ)。
