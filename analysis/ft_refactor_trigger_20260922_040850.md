# 拡張双対単体法の線形代数コア分析(自己改善ループ 反復#8、新キャンペーン第1回) -- 対象: Forrest-Tomlin 更新の再分解トリガー(`ft_refactor_trigger`)。FTRAN/BTRAN の 1 回あたり単価が HiGHS の 2〜9 倍なのは疎性活用の差ではなく、eta 鎖を 300〜2,700 本まで伸ばす「コスト無視の再分解トリガー」が主因。HiGHS 型の synthetic clock 相当をプロトタイプすると主要 24 問題合計で -6%(鎖長が支配的な問題は -20〜-38%)、目的値は全問題一致

対象: NETLIB93 全体(§1)と、そのうち主要 24 問題での段階別計装(§2〜§4)。
基点: `main` = 2206a00。4 コア Intel Xeon 2.80GHz、rustc 1.94.1、highspy 1.15.1(Python 3.11)。
選定段: **(b)** 「FTRAN/BTRAN 1 回あたりの平均時間が HiGHS に対して大きく劣る」。(a) の数値的破綻は 93 問題で確認されず(§1.1)。(c) の未実装項目のうち「更新回数上限に応じた再分解トリガーの調整」が本件の対処箇所に一致する。
参照: `docs/lu_comparison_enomoto_vs_highs.md`(HiGHS `HFactor` との構造比較、既存)。本分析はそこで「参考度: 中」とされていた項目のうち、計測で最も効くものを特定した。

全数値はこのセッションでこのコンテナ上で実測。リポジトリ本体のソースは無変更で、計装・プロトタイプはすべて scratchpad 内のコピー(`src_prof/`、`CARGO_TARGET_DIR` 分離、`PYDIR` で `_core` を切り替え、各実行で `_core.__file__` を stderr に出して取り違えを確認)にのみ当てた(§7)。推定で埋めた箇所はそう明記する。

## 0. 結論(要約)

1. **正答性・数値健全性(選定段 a)は 93/93 問題で問題なし**: status 不一致 0、目的値相対誤差 >1e-6 が 0、`update_verify` による棄却 8 回/100,147 反復、`try_update` の pivot 棄却 1 回、illcond 0、`FT_MIN_PIVOT` 起因の再分解 1 回。DSE 重みの相対誤差 ≥100% は dfl001 で 94/23,406 反復(0.4%)、pilot で 7 回のみ。**LU 分解の破綻は存在しない**ので (a) は該当なし(§1.1)。
2. **再分解回数は HiGHS より圧倒的に少ない(選定段 b の前半は該当なし)**: 93 問題合計で本ソルバ 395 回 vs HiGHS 1,686 回(反復数はほぼ同じ 100,147 vs 100,938)。76/93 問題で本ソルバの再分解回数は HiGHS の 1/4 未満、68 問題は 2 回以下。「再分解が多すぎる」問題は 0 件(§1.2)。
3. **FTRAN/BTRAN の 1 回あたり単価は HiGHS の 2〜9 倍(選定段 b の後半に該当)**: 同一問題・同程度の基底サイズで、80bau3b FTRAN 25.1us vs 2.7us(9.3x)、stocfor2 22.9 vs 3.1(7.4x)、bnl2 19.6 vs 5.2(3.8x)、greenbeb 26.4 vs 8.8(3.0x)、d2q06c 45.1 vs 17.5(2.6x)、dfl001 98.8 vs 38.2(2.6x)。BTRAN も同傾向(§2.1)。2 回の独立実行で単価の差は ±3% 以内。
4. **機序は「eta 鎖の長さ」であり、疎性活用の差ではない**(§2.2〜§2.4)。
   - 本ソルバの再分解トリガー(`src/simplex/extended_dual.rs:3320-3330`、`ft_max_updates = 3m`、`FT_BUMP_LIMIT_FACTOR = 64m`、drift 検査)は**解のコストを一切見ない**。結果、更新回数のピーク(`max_update_streak`)は 80bau3b 1,646、d2q06c 1,440、stocfor2 1,295、bnl2 880 と、HiGHS の平均再分解間隔(50〜140 反復、synthetic clock 制御)の 10〜25 倍。
   - FTRAN 単価は更新回数に比例して伸びる: 80bau3b は更新 <100 回で 7us、200-399 回で 10.6us、800 回超で **53us**(HiGHS 2.7us)。stocfor2 は 7us → 60us。この伸びの実体は R-eta 段(FTRAN では `ftran_through_l_and_r_into` / `solve_sparse_into` 内の `r_etas` 走査、`lu.rs:1938-1947` / `2004-2010`)で、80bau3b では FTRAN 25.1us のうち R 段 13.2us(平均 609 本の R-eta、非零 9,449 個を毎回内積)、U 段 8.6us。R-eta の適用は gather 型(`z[p] -= r·z`)なので零スキップが効かず、鎖長に厳密に比例する。
   - 一方、結果ベクトルの密度は HiGHS とほぼ同じ(80bau3b: 本ソルバ 8.6% / HiGHS 1%、bnl2 25% / 2%、greenbeb 37% / 11%、d2q06c 45% / 13%、dfl001 49% / 10% — 本ソルバの方が密なのは長い鎖の fill による)。HiGHS が hyper-sparse 経路を取る割合(80bau3b 100%、bnl2 96%、stocfor2 89%)に相当する疎性は入力側には存在しており、本ソルバの U 段でも 74〜94% の eta は零スキップされている。**入力の疎性を捨てているのではなく、鎖が長いために出力が密になり R 段が重い**。
   - 参考: 更新回数 0〜9 回の区間に限れば本ソルバの FTRAN 単価は HiGHS と同程度〜2 倍(80bau3b 7.3 vs 2.7、greenbea 6.5 vs 8.1、d2q06c 20.0 vs 17.5、pilot87 51.5 vs 63.3)。つまり基本の疎三角解法の実装品質は HiGHS 級で、差はほぼ全部が鎖長。
5. **なぜ鎖が伸びるのを許しているのか**: 本ソルバの `factorize`(Markowitz、BTreeMap ベース)は HiGHS の INVERT より **5〜25 倍遅い**(dfl001 36.8ms vs 1.5ms、80bau3b 1.81 vs 0.14、greenbeb 4.79 vs 0.50、pilot 16.7 vs 3.7、pilot87 79.9 vs 14.1、§4)。fill-in は HiGHS と同程度〜1.4 倍(§4)なので分解**品質**の差ではなく、`docs/lu_comparison_enomoto_vs_highs.md` §3.1 既知の単価差。再分解が高いから既存トリガーが「できるだけ再分解しない」方向に調整されてきた(`FT_MAX_UPDATES` の docs 参照)が、その結果 1 反復あたりの解法コストが再分解 1 回分を何度も超えている問題が多数ある。
6. **プロトタイプ(HiGHS `HEkk::updateFactor` の synthetic clock 相当、`ENOMOTO_SYNTH_CLOCK=<factor>`)**: 「直近の再分解以降に BTRAN+FTRAN+DSE-FTRAN に費やした壁時計 ≥ factor × 直近の再分解時間、かつ更新回数 ≥ 50」で再分解。主要 24 問題(93 問題合計時間の 96% を占める)の合計(2 回実行の最小値): **off 50.69s → f=2 47.88s (-5.5%) / f=4 47.80s (-5.7%)**。鎖長支配の問題で大きく改善: stocfor2 0.286→0.179s(-37%)、bnl2 0.208→0.154(-26%)、80bau3b 0.705→0.483(-31%)、scfxm3 0.091→0.068(-25%)、degen3 0.468→0.368(-21%)、greenbea 0.633→0.478(-24%)、d2q06c 2.842→2.325(-18%)、nesm 0.170→0.145(-15%)。退行: maros-r7 +1〜3%、pilot.ja +7%(f=4)、grow22 +2〜20%(f=1,2)。**24 問題すべてで目的関数値は off と一致(相対差 <1e-9)、status 変化なし**。反復数は再分解タイミングの違いで ±5% 動く(dfl001 23,406→23,442、pilot 3,666→3,412、greenbeb 5,120→4,559、fit2p 5,951→5,816)。HiGHS 同等の f=1 は本ソルバでは**逆効果**(50.69→51.25s、dfl001 の再分解時間 2.1s→5.8s): 再分解単価が 5〜25 倍高いので HiGHS より長い間隔が最適になる(§5)。
7. **副次的発見(FT 更新 1 回あたりのコスト)**: `commit_update` の `Vec::remove` + `slot_pos` 再番号付け(`lu.rs:2381-2384`)が更新コストの 40〜80%(80bau3b 25.1us 中 16.7us、fit2p 40.8 中 25.7、平均 memmove 要素数 600〜1,500)。HiGHS `updateFT` は `u_pivot_index[p_logic] = -1` の**墓石**方式で O(1)。同方式をプロトタイプ(`ENOMOTO_FT_TOMBSTONE=1`)すると FT 更新フェーズは半減(24 問題 ft_update 合計 2.46s→1.31s)し全問題でビット同一だが、墓石を飛ばす分岐で U 段の解法が 5〜15% 遅くなり**正味は -1%(50.20→49.71s)**。提案 1 で鎖が短くなれば墓石数も減り相性が良くなるので、提案 1 の後に組み合わせて再評価すべき(§3)。

## 1. 93 問題全体の状況(`ENOMOTO_PROF_PHASES_EXT=1`、2 回実行、HiGHS は `log_dev_level=2` の rebuild 行を計数)

### 1.1 正答性・数値健全性(選定段 a の確認)

| 指標 | 値 |
|---|---|
| status 不一致(ours ≠ optimal または HiGHS ≠ kOptimal) | 0 / 93 |
| 目的値相対誤差 >1e-6 | 0 / 93 |
| 再分解原因(93 問題合計 395 回) | verify 8 / try_update 1 / bump 44 / drift 342 / d_drift 0 / illcond 0 / max_updates 0 / infeas_check 0 |
| DSE 重み相対誤差 ≥100% の反復 | dfl001 94 回(0.4%)、pilot 7 回、他 0 |
| `factorize` が None(特異)を返した回数 | 0 |

`ENOMOTO_PROF_UPDATE_VERIFY` / `ENOMOTO_DEBUG_ETA_DENSITY` は古典経路(`simplex.rs`)にしか配線されていないため(`grep` で確認、`extended_dual.rs` には無い)、拡張経路の verify 統計は `PROF_PHASES_EXT` の `verify=` カウンタと、§2 の計装で代替した。

### 1.2 再分解回数と反復数(本ソルバ vs HiGHS、上位問題)

| 問題 | ours 反復 | HiGHS 反復 | ours 再分解 | HiGHS 再分解(rebuild) | ours 更新ピーク `max_update_streak` | HiGHS 平均再分解間隔 |
|---|---|---|---|---|---|---|
| dfl001 | 23,406 | 18,117 | 50 | 186 | 2,735 | 97 |
| pilot87 | 6,463 | 8,369 | 55 | 59 | 300 | 141 |
| maros-r7 | 2,443 | 2,457 | 26 | 26 | 990 | 94 |
| fit2p | 5,951 | 4,907 | 23 | 42 | 690 | 116 |
| d2q06c | 6,397 | 5,295 | 27 | 66 | 1,440 | 80 |
| pilot | 3,666 | 4,815 | 30 | 62 | 295 | 77 |
| greenbeb | 5,120 | 4,620 | 10 | 64 | 1,200 | 72 |
| 80bau3b | 3,157 | 3,097 | 6 | 34 | 1,646 | 91 |
| greenbea | 2,555 | 2,524 | 23 | 41 | 905 | 61 |
| degen3 | 1,905 | 2,098 | 2 | 25 | 1,035 | 83 |
| stocfor2 | 1,649 | 821 | 1 | 13 | 1,295 | 63 |
| bnl2 | 1,079 | 1,069 | 1 | 19 | 880 | 56 |
| scfxm3 / ship12l / sctap3 / woodw / sierra / ganges / cycle | 300〜1,000 | 同程度 | **0** | 8〜63 | = 反復数 | 46〜81 |
| 93 問題合計 | 100,147 | 100,938 | 395 | 1,686 | — | — |

HiGHS の再分解は 91% 以上が "Synthetic clock"(コストベース)で、"Update limit"(回数上限 = 5,000 既定)はほぼ発火しない。本ソルバは 342/395 が drift 検査、44 が eta-fill 上限、回数上限 `ft_max_updates(m) = 3m` は 0 回。**本ソルバに「解法コストが再分解コストを上回ったら再分解」という判断は存在しない**。

### 1.3 拡張双対ループの時間内訳(93 問題合計、ext wall 46.9s)

| フェーズ | 合計 | 比率 |
|---|---|---|
| refactor | 8.29s | 17.7% |
| dse_update(DSE 用 FTRAN 含む) | 6.07s | 12.9% |
| bfrt | 5.11s | 10.9% |
| ftran(入基底列) | 4.56s | 9.7% |
| btran(rho_p) | 4.54s | 9.7% |
| price | 4.25s | 9.1% |
| xb_update | 3.60s | 7.7% |
| ft_update | 2.62s | 5.6% |
| chuzr / chuzc1 / dual_update | 5.08s | 10.8% |
| **線形代数コア合計(refactor+dse+ftran+btran+ft_update)** | **26.1s** | **55.6%** |

線形代数 5 フェーズが半分強。うち反復ごとの三角解法(dse+ftran+btran)32.3%、再分解 17.7%、更新 5.6%。

## 2. 三角解法の段階別計装(24 問題、`ENOMOTO_PROF_LU=1`、scratch コピーのみ)

計装内容: `FtLu::solve_into*` / `solve_sparse_into*` を L 段・R 段・U 段・permute に分割計時、`solve_transpose_into*` を permute・U^T 段・R 段・L^T 段に分割、各呼び出しの出力非零数、R-eta 本数・非零数、U 段でスキップされた eta 数、U^T 段で `needed` になった eta 数、更新回数ごとの単価ヒストグラム、`commit_update` の 4 段階(r_vec 構築 / `Vec::remove` / row_owners 更新 / push)、`refactorize` の分解時間と L/U 非零数。計装込みの単価は非計装比 +5〜8%(`Instant::now` が 1 解法あたり 4〜5 回入るため)だが、**比率とカウンタは 2 回の実行で同一**。

### 2.1 1 回あたり単価: 本ソルバ(run1/run2)vs HiGHS(`highs_analysis_level=63` の `SimplexInner-time` クロック、同一 .mps、HiGHS 自身の presolve 後)

| 問題 | m(ours/HiGHS) | FTRAN us ours | HiGHS | 比 | BTRAN us ours | HiGHS | 比 | FT 更新 us ours | HiGHS | 再分解 ms ours | HiGHS |
|---|---|---|---|---|---|---|---|---|---|---|---|
| dfl001 | 4380/3953 | 98.8/97.2 | 38.2 | 2.6 | 91.7/88.9 | 43.8 | 2.1 | 48.6/47.2 | 3.47 | 36.8/37.0 | 1.51 |
| pilot87 | 1954/1700 | 92.4/92.5 | 63.3 | 1.5 | 118.7/117.3 | 66.6 | 1.8 | 42.5/41.0 | 13.9 | 79.9/79.4 | 14.1 |
| fit2p | 3000/3000 | 39.6/39.4 | 41.2 | 1.0 | 79.5/78.5 | 15.2 | 5.2 | 40.9/40.1 | 9.39 | 6.84/6.93 | 0.92 |
| d2q06c | 1973/1759 | 45.1/46.2 | 17.5 | 2.6 | 41.3/43.0 | 19.0 | 2.2 | 27.0/27.9 | 2.27 | 6.30/6.46 | 0.95 |
| pilot | 1316/1155 | 48.4/47.6 | 27.1 | 1.8 | 56.4/56.5 | 27.7 | 2.0 | 24.6/24.7 | 6.16 | 16.7/16.6 | 3.71 |
| maros-r7 | 2152/2152 | 79.6/81.4 | 44.2 | 1.8 | 67.3/68.2 | 38.2 | 1.8 | 35.3/35.2 | 7.57 | 13.0/13.0 | 5.01 |
| greenbeb | 1585/1049 | 26.4/25.9 | 8.8 | 3.0 | 22.8/22.4 | 10.2 | 2.2 | 19.0/18.7 | 1.39 | 4.79/4.80 | 0.50 |
| 80bau3b | 1989/1640 | 25.1/25.2 | 2.7 | **9.3** | 18.4/18.9 | 3.9 | 4.7 | 25.0/25.2 | 0.53 | 1.81/1.75 | 0.14 |
| greenbea | 1587/1047 | 29.2/28.8 | 8.1 | 3.6 | 22.6/22.3 | 9.3 | 2.4 | 20.3/19.6 | 1.12 | 1.13/1.09 | 0.36 |
| degen3 | 1412/1405 | 32.2/34.6 | 10.8 | 3.0 | 29.2/32.9 | 12.7 | 2.3 | 24.6/26.2 | 1.44 | 4.04/4.08 | 0.44 |
| stocfor2 | 1766/1196 | 22.9/23.5 | 3.1 | **7.4** | 23.6/24.3 | 2.1 | **11.2** | 22.3/22.9 | 0.37 | 1.04/0.92 | 0.08 |
| bnl2 | 1139/964 | 19.6/19.0 | 5.2 | 3.8 | 15.5/15.4 | 5.1 | 3.0 | 17.1/17.1 | 0.59 | 0.66/0.74 | 0.15 |
| scfxm3 | 799/703 | 10.4/10.2 | 3.3 | 3.2 | 8.7/8.8 | 2.7 | 3.2 | 12.1/11.4 | 0.47 | (初回のみ) | 0.15 |
| sctap3 | 1344/1344 | 9.9/9.5 | 1.2 | **8.3** | 9.7/9.8 | 0.9 | **10.8** | 15.1/14.3 | 0.20 | (初回のみ) | 0.06 |
| ship12l | 686/609 | 5.7/5.7 | 1.9 | 3.0 | 4.7/4.3 | 2.1 | 2.2 | 9.0/9.0 | 0.53 | (初回のみ) | 0.05 |

注: HiGHS の m は HiGHS 自身の presolve 後、本ソルバの m は本ソルバの presolve 後で 5〜50% 大きい。m 比を補正しても 80bau3b・stocfor2・sctap3 の 5〜10 倍差は残る。HiGHS の FTRAN クロックは `ftranL+ftranU(FT eta 含む)` で、本ソルバの L+R+U+perm に対応する。

### 2.2 FTRAN の段階内訳(run1、us/回)と鎖・密度統計

| 問題 | L | R | U | perm | 合計 | 入力 nnz | 出力 nnz(密度) | R-eta 本数 | R 非零合計 | U 段スキップ率 | HiGHS 出力密度 | HiGHS hyper 経路率 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dfl001 | 10.9 | 30.1 | 52.1 | 5.6 | 98.8 | 698 | 2,146 (49%) | 365 | 21,826 | 51% | 10% | 18% |
| pilot87 | 20.7 | 17.7 | 51.8 | 2.3 | 92.4 | 306 | 1,117 (57%) | 100 | 13,522 | 43% | 60% | 1% |
| d2q06c | 4.3 | 16.7 | 22.1 | 2.1 | 45.1 | 240 | 879 (45%) | 322 | 12,483 | 55% | 13% | 28% |
| maros-r7 | 6.4 | 34.8 | 36.6 | 1.9 | 79.6 | 205 | 1,016 (47%) | 238 | 24,484 | 53% | 17% | 4% |
| greenbeb | 2.6 | 9.1 | 13.2 | 1.5 | 26.4 | 174 | 585 (37%) | 315 | 7,063 | 63% | 11% | 32% |
| **80bau3b** | 1.6 | **13.2** | 8.6 | 1.8 | 25.1 | 51 | 171 (8.6%) | **609** | 9,449 | 91% | 1% | 100% |
| greenbea | 2.1 | 13.4 | 12.4 | 1.4 | 29.2 | 117 | 406 (26%) | 294 | 10,548 | 74% | 11% | 22% |
| degen3 | 2.4 | 13.8 | 14.7 | 1.3 | 32.2 | 142 | 523 (37%) | 368 | 10,649 | 63% | 7% | 66% |
| **stocfor2** | 1.4 | **11.7** | 8.5 | 1.4 | 22.9 | 56 | 316 (18%) | **550** | 8,922 | 82% | 2% | 89% |
| bnl2 | 1.1 | 9.2 | 8.3 | 1.0 | 19.6 | 58 | 285 (25%) | 387 | 7,360 | 75% | 2% | 96% |
| scfxm3 | 0.7 | 5.3 | 3.7 | 0.7 | 10.4 | 41 | 165 (21%) | 460 | 3,730 | 79% | 3% | 93% |
| sctap3 | 1.1 | 3.2 | 4.6 | 1.0 | 9.9 | 35 | 220 (16%) | 231 | 2,810 | 84% | 0.4% | 100% |
| ship12l | 0.5 | 2.8 | 1.7 | 0.6 | 5.7 | 18 | 51 (7%) | 500 | 1,911 | 93% | 1% | 100% |

読み方: 80bau3b / stocfor2 / bnl2 / scfxm3 / ship12l のように**入力も出力も疎(≤25%)な問題では R 段が FTRAN の 45〜55% を占め、それは R-eta 本数(=更新回数、一度も再分解しないので反復数まで伸びる)に比例**している。L 段(GP 疎解法)は 0.5〜2us で HiGHS 並み。U 段は零スキップが 75〜93% 効いており、残りは `u_seq` の全 eta を走査する固定コスト(m 回の分岐)と、鎖の fill で生じた非零の処理。

BTRAN(§7 生データ `parse_lu.py` 出力参照)も同型で、80bau3b は perm 1.6 / U^T 7.4 / R 5.0 / L^T 4.5 = 18.4us、U^T 段で `needed` になった eta は 35.5 本/1,989 本(1.8%)にすぎないが、`stamps` 初期化の O(m) 走査(`lu.rs:1835-1839`)と `u_seq` 全走査の分岐が固定コストとして残る。L^T 段(`l_transpose_solve_into`、密走査)は m=2,000 で 4.5us。

### 2.3 更新回数と単価の関係(run1、FTRAN us/回、括弧は呼び出し数)

| 問題 | 更新 0 | 1-9 | 10-49 | 50-99 | 100-199 | 200-399 | 400-799 | 800+ | HiGHS |
|---|---|---|---|---|---|---|---|---|---|
| 80bau3b | 12.3 (38) | 7.3 (162) | 6.8 (418) | 7.6 (496) | 9.1 (914) | 10.6 (1,640) | 18.3 (2,362) | **53.1 (3,076)** | 2.7 |
| stocfor2 | 12.3 (8) | 7.6 (36) | 8.2 (176) | 7.6 (312) | 6.9 (594) | 8.0 (934) | 10.2 (1,202) | **60.0 (1,402)** | 3.1 |
| bnl2 | 8.6 | 6.3 | 6.0 | 7.8 | 8.8 | 7.2 | 29.0 (1,224) | 67.8 (228) | 5.2 |
| greenbea | 10.7 | 6.5 | 7.3 | 9.8 | 13.1 | 27.1 | 57.6 (2,038) | 97.0 | 8.1 |
| d2q06c | 23.6 | 20.0 | 22.8 | 28.1 | 36.2 | 57.9 (5,248) | 64.6 (3,058) | 62.9 | 17.5 |
| greenbeb | 16.9 | 12.1 | 13.0 | 15.6 | 19.7 | 31.8 (4,636) | 39.1 | 35.2 | 8.8 |
| dfl001 | 75.2 | 58.2 | 59.4 | 64.3 | 77.6 | 113.7 (21,462) | 172.4 (11,857) | 51.0* | 38.2 |
| pilot87 | 58.5 | 51.5 | 67.0 | 84.2 | 113.7 | 135.6 | - | - | 63.3 |

(* dfl001 の 800+ は主ループ末期の `polish` 段で入力が極端に疎な区間。)HiGHS が実際に運転している 50〜140 更新の区間では、本ソルバの単価は HiGHS の 1〜2.5 倍。**差の大部分は HiGHS が決して踏み込まない 200 更新超の区間で発生**している。BTRAN も同じ形(80bau3b: 10.6us → 29.2us、stocfor2: 7.7 → 59.8)。

### 2.4 疎/密切り替えの実態

- `should_use_dense_solve`(`lu.rs:1788`、閾値 `DENSE_RHS_FRACTION = 0.4`、入力 nnz のみで判定): 24 問題で FTRAN 呼び出しの 65〜70% が疎経路(入基底列)、残りは DSE の `tau = B^{-1} rho`(`extended_dual.rs:3121`、rho は密なので常に密経路)と BFRT の combined。HiGHS は FTRAN-DSE でも hyper 経路を 5〜66% 使うが、それは結果密度が低い問題(degen3 66%、bnl2 96%)に限られ、本ソルバがそれらで損している量は R 段に比べれば小さい(推定: U 段の固定走査 + perm + `scratch.fill` で 1 回 2〜4us、80bau3b の 25us 中 10〜15%)。
- eta の Sparse/Dense 切替(`DENSE_ETA_FRACTION = 0.4`)は 24 問題で Dense 表現に落ちた eta が確認できず(`OffDiag::Dense` は fill_count 経由でしか見えないため間接確認: 平均 U-eta 非零数は最大でも 180/m=1,954 = 9%)。Netlib では実質未使用。
- HiGHS の `expected_density` 移動平均型の事前切替(`HFactorConst.h`: `kHyperFtranL=0.15, kHyperFtranU=0.10, kHyperBtranL=0.10, kHyperBtranU=0.15, kHyperCancel=0.05`)に相当するものは無いが、上記の通り本件では主因ではない。

## 3. Forrest-Tomlin 更新 1 回あたりのコスト(run1、us)

| 問題 | 呼び出し | r_vec 構築 | `Vec::remove`+再番号 | row_owners 更新 | push/pack | 合計 | R 非零 | U 非零 | memmove 要素数 | HiGHS UPDATE_FACTOR |
|---|---|---|---|---|---|---|---|---|---|---|
| dfl001 | 23,404 | 8.0 | **23.8** | 6.4 | 10.4 | 48.6 | 97 | 113 | 1,432 | 3.47 |
| fit2p | 5,949 | 7.3 | **25.8** | 1.7 | 6.1 | 40.9 | 116 | 32 | 1,557 | 9.39 |
| 80bau3b | 3,156 | 2.8 | **16.7** | 1.6 | 4.0 | 25.0 | 23 | 45 | 1,040 | 0.53 |
| stocfor2 | 1,648 | 3.0 | **14.8** | 0.8 | 3.8 | 22.3 | 39 | 41 | 919 | 0.37 |
| sierra | 500 | 1.2 | **10.8** | 0.1 | 1.5 | 13.6 | 4 | 6 | 696 | 0.19 |
| pilot87 | 6,459 | 5.7 | 14.8 | 12.6 | 9.4 | 42.5 | 130 | 180 | 870 | 13.9 |
| 25fv47 | 3,113 | 1.9 | 3.8 | 1.8 | 2.8 | 10.3 | 45 | 63 | 264 | (未取得) |

`Vec::remove(seq_pos)`(`lu.rs:2381`)は平均で `u_seq` の半分(≈ m/2 要素 × 32 バイトの `UEta`)を memmove し、続く `slot_pos` 再番号付けループ(`:2382-2384`)が同数の書き込みをする。加えて `r_vec` / `off_diag` の構築(`:2368-2369`, `:2405-2406`)は `(0..m)` の密走査で O(m)。HiGHS `updateFT`(`HFactor.cpp:2299-2345`)は `u_pivot_index[p_logic] = -1` の墓石を残して末尾に追加するだけで、削除は転置索引 `ur_*` 経由で触れる eta だけを更新する(本ソルバの `row_owners` と同目的)。

墓石プロトタイプ(`ENOMOTO_FT_TOMBSTONE=1`、`lu.rs` の `commit_update` / `u_solve_into` / `u_transpose_solve_into` に `slot == usize::MAX` を飛ばす分岐を追加)の結果(24 問題、2 回実行):

| | off | tombstone |
|---|---|---|
| 24 問題合計時間(2 回の最小値) | 50.20s | 49.71s(-1.0%) |
| ft_update フェーズ合計 | 2.46s | 1.31s(-47%) |
| 更新単価(80bau3b / fit2p / dfl001) | 25.1 / 40.8 / 47.0 us | 8.7 / 14.5 / 24.8 us |
| FTRAN U 段単価(80bau3b / dfl001) | 8.9 / 51.1 us | 10.7 / 54.4 us(+5〜20%) |
| BTRAN U^T 段単価(80bau3b / dfl001) | 7.5 / 32.3 us | 10.5 / 36.8 us |
| 目的関数値 | — | 24/24 でビット同一、反復数・再分解回数も同一 |

更新は半減するが、鎖が 1,000 本を超える問題では `u_seq` に墓石が 1,000 個並び、毎回の U 段・U^T 段走査がその分だけ遅くなって相殺する。**鎖を短く保つ提案 1 と組み合わせて初めて正味で効く**(鎖 ≤ 100 なら墓石は m の数%)。

## 4. LU 分解(`factorize`)の単価と fill-in(run1)

| 問題 | 回数 | ms/回 | m | nnz(B) | L 非零 | U 非対角 | fill=(L+U+m)/nnz(B) | HiGHS INVERT ms/回 | HiGHS fill(invert_fill_factor 平均) | 単価比 |
|---|---|---|---|---|---|---|---|---|---|---|
| dfl001 | 51 | 36.8 | 4,380 | 12,244 | 8,933 | 9,401 | 1.86 | 1.51 | 1.32 | **24x** |
| pilot87 | 56 | 79.9 | 1,954 | 15,272 | 12,116 | 23,722 | 2.48 | 14.1 | 2.18 | 5.7x |
| pilot | 31 | 16.7 | 1,316 | 9,971 | 5,240 | 12,371 | 1.90 | 3.71 | 1.76 | 4.5x |
| maros-r7 | 27 | 13.0 | 2,152 | 14,237 | 1,825 | 17,725 | 1.52 | 5.01 | 1.39 | 2.6x |
| fit2p | 24(bordered 23) | 6.8 | 3,000 | 31,419 | 87 | 28,364 | 1.00 | 0.92 | — | 7.4x |
| d2q06c | 28 | 6.3 | 1,973 | 7,563 | 2,184 | 5,434 | 1.27 | 0.95 | 1.19 | 6.6x |
| greenbeb | 11 | 4.8 | 1,585 | 7,536 | 2,598 | 5,070 | 1.23 | 0.50 | 1.14 | 9.6x |
| degen3 | 3 | 4.0 | 1,412 | 9,856 | 2,259 | 6,930 | 1.08 | 0.44 | 1.01 | 9.2x |
| 80bau3b | 7 | 1.8 | 1,989 | 3,535 | 602 | 1,215 | 1.08 | 0.14 | — | 13x |
| greenbea | 24 | 1.1 | 1,587 | 2,596 | 270 | 899 | 1.06 | 0.36 | 1.11 | 3.1x |

fill-in の定義は両者で完全には一致しない可能性がある(HiGHS の `invert_fill_factor` は `grep_kernel` 行の 7 列目、本ソルバは L+U+対角 / 基底非零)が、桁は同じで本ソルバが 1.0〜1.4 倍。**ピボット戦略(Markowitz + 安定性床 0.25)は fill の面で HiGHS(閾値 0.1、searchLimit 8)と同等**であり、単価差は既知のデータ構造(BTreeMap/BTreeSet、`docs/lu_comparison_enomoto_vs_highs.md` §3.1)による。`FtLu::new`(eta 列構築)は 0.1〜1.0ms で分解の 2〜15%。`factorize_bordered` が使われたのは fit2p のみ、`factorize_diagonal` は初回スラック基底のみ。

再分解単価が HiGHS の 5〜25 倍ある以上、HiGHS と同じ頻度で再分解すれば損をする(§5 の f=1)。したがって本ソルバの最適な再分解間隔は HiGHS より長く、**それでも現状(間隔 = 反復数、または 3m)は長すぎる**、というのが本分析の結論。

## 5. プロトタイプ: コストベース再分解トリガー(HiGHS `HEkk::updateFactor` の synthetic clock 相当)

HiGHS の実装(`HEkk.cpp:3075-3090`、本セッションで master から取得して確認):
```cpp
  bool reinvert_syntheticClock = this->total_synthetic_tick_ >= this->build_synthetic_tick_;
  const bool performed_min_updates = info_.update_count >= kSyntheticTickReinversionMinUpdateCount;  // 50
  if (reinvert_syntheticClock && performed_min_updates) *hint = kRebuildReasonSyntheticClockSaysInvert;
```
`total_synthetic_tick_` は各 FTRAN/BTRAN の中で「触れた eta の非零数 × 定数」を積算した**決定的な演算量カウンタ**(`HFactor.cpp:1717-1718` など: `rhs.synthetic_tick += rhs_synthetic_tick * 15 + (u_pivot_count - num_row) * 10`)、`build_synthetic_tick_` は直近の INVERT の同種カウンタ(`HFactor.cpp:1499`: `num_row * 80 + (LcountX + u_countX) * 60`)。

本プロトタイプ(scratch コピーの `extended_dual.rs` のみ、`ENOMOTO_SYNTH_CLOCK=<factor>`): 決定的カウンタの代わりに**壁時計**で「BTRAN + 入基底 FTRAN + DSE-FTRAN の累計 ≥ factor × 直近の `refactorize` 時間、かつ `update_count ≥ 50`」で再分解(主ループ `extended_dual.rs:3320` の直後に追加、`REFACTOR_CAUSE_MAX_UPDATES` に計上)。壁時計なので実行間で再分解タイミングが揺れうる(実測では 2 回の実行で反復数が一致した問題が大半だが、pilot / greenbeb で数%動いた)。

| 問題 | off(基準) | f=1 | f=2 | f=4 | 再分解回数 off→f=2→f=4 | HiGHS 再分解 |
|---|---|---|---|---|---|---|
| dfl001 | 22.55s | 23.44 | 21.37 | **21.69** | 50→99→71 | 186 |
| pilot87 | 10.38 | 11.20 | 10.73 | **10.26** | 55→58→55 | 59 |
| fit2p | 3.59 | 3.67 | **3.22** | 3.35 | 23→42→30 | 42 |
| d2q06c | 2.84 | 2.45 | **2.33** | 2.40 | 27→47→38 | 66 |
| pilot | 2.26 | 1.95 | 2.02 | **1.94** | 30→32→28 | 62 |
| maros-r7 | 3.68 | 3.80 | 3.77 | 3.70 | 26→31→29 | 26 |
| greenbeb | 1.18 | 0.98 | 1.00 | **0.96** | 10→28→17 | 64 |
| 80bau3b | 0.705 | 0.589 | 0.500 | **0.483** | 6→24→16 | 34 |
| greenbea | 0.633 | 0.515 | **0.478** | 0.489 | 23→34→29 | 41 |
| degen3 | 0.468 | 0.392 | 0.359 | **0.368** | 2→11→7 | 25 |
| 25fv47 | 0.498 | 0.463 | **0.432** | 0.450 | 18→31→24 | (未取得) |
| stocfor2 | 0.286 | 0.205 | 0.184 | **0.177** | 1→16→9 | 13 |
| bnl2 | 0.208 | 0.166 | 0.156 | **0.154** | 1→11→6 | 19 |
| nesm | 0.170 | 0.159 | 0.147 | **0.145** | 3→10→7 | 45 |
| scfxm3 | 0.091 | 0.076 | 0.071 | **0.068** | 0→7→4 | 17 |
| ship12l | 0.082 | 0.083 | 0.076 | **0.074** | 0→10→6 | 14 |
| sctap3 | 0.063 | 0.057 | 0.053 | **0.052** | 0→6→4 | 12 |
| grow22 | 0.221 | 0.265 | 0.233 | 0.224 | 24→31→28 | 27 |
| pilot.ja | 0.335 | 0.333 | 0.320 | 0.361 | 12→15→21 | 21 |
| perold / woodw / sierra / cycle / ganges | 0.158 / 0.147 / 0.048 / 0.063 / 0.046 | ±5% | ±5% | ±5% | | |
| **24 問題合計(2 回の最小値)** | **50.69s** | 51.25 (+1.1%) | **47.88 (-5.5%)** | **47.80 (-5.7%)** | | |

- 目的関数値: 4 設定 × 24 問題すべてで off との相対差 <1e-9(status も全て optimal)。
- 反復数の変化は再分解時の完全再同期で丸めが変わるため(`extended_dual.rs:3419-3430` のコメント通り意図的なフル再同期)。増減はあるが方向性は無い(dfl001 +0.2〜1.4%、fit2p -2%、pilot -7%、greenbeb -11%、grow22 +3%)。
- f=1(HiGHS と同じ「解法時間 = 再分解時間で再分解」)は dfl001 で再分解 172 回・再分解時間 2.1s→5.8s となり逆効果。再分解単価が HiGHS の 24 倍なので当然で、本ソルバの最適 factor は 2〜4(§4 の単価差が縮まれば 1 に近づく)。
- 24 問題は 93 問題合計時間の 96% を占める。残る 69 問題(合計 <2s)は反復数が数百で鎖が短く、この変更の影響は小さい(推定)。

## 6. 原因分類と改善提案

**分類: 数値安定性パラメータの違い(再分解トリガー設計)**。具体的には「更新回数上限に応じた再分解トリガー」が回数・fill・drift のみで、**解法コストと再分解コストの比較を持たない**こと。疎性活用の不足・hyper-sparsity 閾値・更新式の実装ミス・ピボット戦略の違いは、いずれも本件の主因ではないことを計測で除外した(§2.4、§3、§4)。

該当箇所:
- `src/simplex/extended_dual.rs:3320-3330`(主ループ、`ft_max_updates(m) = 3m` と `FT_BUMP_LIMIT_FACTOR * m` の判定)、同 `:4137-4142`(`polish_with_true_bounds` 内の同型判定)、`:586-588`(`ft_max_updates`)、`:577`(`FT_MAX_UPDATES_FACTOR = 3.0`)。
- `src/simplex.rs:401`(`FT_BUMP_LIMIT_FACTOR = 64`)、`:407`(`FT_MAX_UPDATES = 300`)。
- コスト発生源: `src/simplex/lu.rs:1938-1947` / `2004-2010` / `2040-2046`(FTRAN の R-eta 段、gather 型で零スキップ不可)、`:2078-2095`(BTRAN の R 段)、`:1904-1927`(U 段の `u_seq` 全走査)、`:1823-1858`(U^T 段の `stamps` 初期化と全走査)。
- 更新単価: `src/simplex/lu.rs:2381-2384`(`Vec::remove` + `slot_pos` 再番号付け)、`:2368-2369` / `:2405-2406`(O(m) 密走査で r_vec / off_diag 構築)。

### 提案(優先順)

1. **決定的な演算量カウンタによるコストベース再分解トリガー**(効果: 24 問題合計 -5.5〜-5.7%、鎖長支配問題 -15〜-37%; 93 問題では推定 -5% 前後)。
   - `FtLu` に `synthetic_tick: Cell<u64>`(または各 solve が返す演算量)を追加し、R 段は `r_nnz` 合計、U 段は処理した eta の非零数、L 段は reach 集合サイズを積算。`refactorize` 側は `build_tick = 80·m + 60·(l_nnz + u_off)`(HiGHS と同式)を記録。主ループ `extended_dual.rs:3320` の直後と `polish` の `:4137` の直後に `if update_count >= 50 && acc_tick >= FACTOR * build_tick { need_refactor }` を追加。FACTOR は本プロトタイプの壁時計換算で 2〜4 が最適だったが、tick 換算では再計測が必要(本ソルバの分解単価が tick あたり HiGHS より高いので、tick 比では 1 未満に相当する可能性がある)。
   - リスク: 再分解タイミングがピボット経路を変える(目的値は 24 問題で一致したが、93 問題全数で status・目的値・反復数を確認すること)。`scripts/loop_state.json` の反復#7 申し送り「再分解タイミングに触れる変更の前に BIG_M クランプ中に optimal を返さない安全策を入れる」に該当するので、`simplex.rs:2972-3000` 付近の BIG_M フォールバックが発火しないことを `ENOMOTO_DEBUG_EXT_ITERS` で確認する。壁時計ではなく tick を使えば決定性は保たれる。
   - 既存 API・アーキテクチャは不変(トリガーが 1 つ増えるだけ。`REFACTOR_CAUSE_*` に `CLOCK` を追加して `PROF_PHASES_EXT` に出す)。
2. **`commit_update` の墓石化 + 再分解時の自然な圧縮**(効果: ft_update フェーズ -47% = 93 問題合計の約 2.6%、ただし単独では U 段が遅くなり正味 -1%。提案 1 の後なら墓石数が ≤100 に収まり正味で効く見込み。ビット同一で経路を変えない)。`u_seq.remove` を `slot = usize::MAX` の墓石に置換し、`u_solve_into` / `u_transpose_solve_into` / `fill_count` で墓石を飛ばす。`FtLu::new` が毎回作り直すので圧縮は再分解で自動。加えて `r_vec` / `off_diag` 構築の `(0..m)` 密走査を、`e_tilde` / `a_tilde` の非零索引(`u_transpose_solve_into` の `stamps` と `l_solve_sparse_into` の reach から取れる)に置き換えれば O(nnz) にできる(効果は更新 1 回あたり 2〜8us、93 問題合計で <1.5% と推定)。
3. **`factorize` の単価(HiGHS INVERT の 5〜25 倍)**: 提案 1 を入れると再分解回数が 1.5〜3 倍になり、refactor フェーズ(現在 17.7%)が最大の項目になる。`docs/lu_comparison_enomoto_vs_highs.md` §2.1(`buildSimple` 型の一括剥離)・§2.2(前回ピボット順の再利用 `rebuild()`)・§3.1(アクティブ部分行列のフラット配列化)が対応する。反復#10 の分析(`analysis/greenbea_20260921_150000.md`)で「fill 1 個あたり約 0.8us」と実測済みで、本分析の 24 問題でも `ms ≈ 0.3 + 0.0008 × (L+U)` にほぼ乗る。次回以降のキャンペーン項目として独立に扱うべき規模(lu.rs の MarkowitzState 全面改修)。
4. **三角解法の固定 O(m) コスト**(効果: FTRAN/BTRAN 1 回あたり 2〜4us、鎖が短くなった後の残差として 10〜20%): `solve_transpose_into` の `stamps` 初期化(`lu.rs:1835-1839`、BTRAN の入力は単位ベクトルなので `col_perm_inv[r]` 1 点を stamp すれば済む)、permute 2 回、`scratch.fill(0.0)`、`l_transpose_solve_into` の密走査(HiGHS は `lr_*` 転置 L で hyper-sparse、`docs/lu_comparison` §2.6)。いずれもビット同一で実装できるが、提案 1 で鎖が短くなってから再計測して優先度を決めるのが妥当。

**該当なし(計測で除外)**: eta の Sparse/Dense 閾値(`DENSE_ETA_FRACTION`、Netlib では Dense に落ちない)、`DENSE_RHS_FRACTION`(入力密度は HiGHS の hyper 判定と整合)、Markowitz の安定性床・fill(HiGHS と同等)、`update_verify` / `FT_MIN_PIVOT` / drift 検査の閾値(発火回数が少なく健全)。

## 7. 再現手順

環境: `python -m venv .venv && .venv/bin/pip install maturin highspy pytest && . .venv/bin/activate && maturin develop --release && python scripts/setup_netlib_data.py`。

93 問題サーベイ(本ソルバ、2 回):
```
for p in $(cat .netlib_cache/problems.txt); do ENOMOTO_PROF_PHASES_EXT=1 .venv/bin/python <scratch>/prof_one.py $p > survey1/$p.json 2> survey1/$p.err; done
```
`prof_one.py` は `benchmark_highs._load_lp` / `_build_our_model` をそのまま使い `model.solve(root_solver=None)` を 1 回呼ぶだけ(`PYDIR` 環境変数で `_core` の場所を切り替える)。

HiGHS 側(再分解回数と反復数): `highspy.Highs()` に `output_flag=True, log_dev_level=2` で `readModel`→`run`、ログの `DuPh*/PrPh*` 行末の "Synthetic clock" / "Update limit reached" を計数。線形代数クロック・密度・hyper 経路率・fill: `log_dev_level=3, highs_analysis_level=63` で `SimplexInner-time` / `FactorLevel2-time` / `FTRAN performed N times` / `grep_kernel` 行を読む(`<scratch>/highs_nla.py`、`parse_highs_nla.py`)。

段階別計装(scratch コピー `src_prof/` に `patch_prof_lu.py` を適用、`CARGO_TARGET_DIR=<scratch>/target_prof cargo build --release`、`lib_core.so` を `src_prof/python/enomoto_solver/_core.cpython-311-x86_64-linux-gnu.so` にコピー):
```
PYDIR=<scratch>/src_prof/python ENOMOTO_PROF_LU=1 ENOMOTO_PROF_PHASES_EXT=1 .venv/bin/python prof_one.py 80bau3b
```
出力の `PROF_LU factorize/ftran/btran/update` 行と `ftran by update_count` ヒストグラムが §2〜§4 の元データ。

プロトタイプ: 同コピーに `patch_clock.py`(`ENOMOTO_SYNTH_CLOCK=<factor>`、`ENOMOTO_SYNTH_MIN_UPDATES` 既定 50)、`patch_tomb.py`(`ENOMOTO_FT_TOMBSTONE=1`)を重ねて適用し、24 問題 × {off,1,2,4} × 2 回、および 24 問題 × {off,tombstone} × 2 回を実行(`parse_clock.py`、§3 のインライン集計)。

HiGHS 参照コード(master、本セッションで取得): `highs/simplex/HEkk.cpp` `updateFactor`(synthetic clock 判定)、`highs/util/HFactor.cpp` `updateFT`(墓石)・`ftranU`/`btranU`(密度判定と tick 積算)・`buildFinish`(build tick)、`highs/util/HFactorConst.h`(閾値定数)。

計装パッチ・プロトタイプはいずれも scratchpad 内のコピーにのみ適用し、リポジトリ本体のソースは無変更(`git status` クリーン、本ファイルのみ追加)。
