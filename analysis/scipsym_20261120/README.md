# SCIP-SYM: 対称性の差 (cod105・fhnw-binpack4-4、2026-11-20)

依頼 (親 session_01HrNYnHHEAD5vXdU4MHQrtc): (a) cod105 は当方 1 スレッド 242 秒・根の下界 −18.29、SCIP 64 秒・−13.53。SCIP の
misc/usesymmetry のビット 1 だけで 48 秒、ビット 4 (SST) だけで 55 秒、ビット 2 だけでは 333 秒 (vsscip_slow_20261118/tree2 9 節)。
(b) fhnw-binpack4-4 は SCIP 12 秒、決め手は misc/usesymmetry (hang_20261117)。当方の ORBPP・SST・SYMDYN で何が働いて何が欠けているかを切り分け、
同じ型の問題に効く形で試作する。

- 機械: 4 CPU のコンテナ。土台: このブランチの ff9948d04。SCIP は pyscipopt 6.3.0 (SCIP 10.0.2)。SCIP の源は v10.0.2 の prop_symmetry.c を読んだ。
- 台本: `scipab.py <mps> <秒> <名前> [パラメータ=値 ...]` (SCIP を 1 設定で、ログと統計は `logs/<問題>_<名前>.log(.stat)`)、
  `one.sh` (当方の 1 回、hang_20261117 の写し)、`pplexscan.py` (P・M の走査 → `pplexscan.jsonl`)、`fuzz_pplex.py`・`fuzz.sh` (誤答の確かめ → `fuzz/`)、
  `cfg.py`・`plan.py`・`run.py`・`summ.py` (広い計測、6 節)。

## 1. SCIP は cod105 で何をしているか (事実、種 1)

| 設定 (`limits/nodes=1` で前処理と根だけ、20 秒) | 前処理後の列 | 根の下界 | 足した対称性の制約 |
|---|---|---|---|
| usesymmetry=0 (5 秒で打ち切り) | 1024 | (根に届かず) | なし |
| usesymmetry=4 (SST だけ) | 472 | −13.66 | orbitope_pp 55・symresack 1 |
| usesymmetry=1 (対称性の制約だけ) | 472 | −13.66 | orbitope_pp 55 |
| 1 + detectsubgroups・detectorbitopes・addweaksbcs・detectdoublelex・addsymresacks のどれか 1 つを切る | 472 | −13.64〜−13.66 | orbitope_pp 55 |
| 既定 (7) + プロービングを切る (`propagating/probing/maxprerounds=0`) | 969 | −18.22 | orbitope_pp 55 |

- 既定 (120 秒): 41.5 秒・3 ノード。usesymmetry=4: 45.5 秒・3 ノード。対称性なし: 1024 列のまま、根の LP 11864 反復 4.8 秒。
- 前処理の内訳 (usesymmetry=4): symmetry が 41 列を固定し 56 本の制約を足す → orbitope_pp の伝播が 16 列 → **プロービングが 495 列** を固定。
- どのビットでも同じ 55 本の orbitope_pp が出る源は、prop_symmetry.c の `componentPackingPartitioningOrbisackUpgrade`
  (「詰め込み・分割の orbisack への格上げ」、lexred が有効なら SST・対称性の制約のどちらの道でも呼ばれる):
  1. `tryGenerateInvolutions`: 成分の生成元のうち対合 (互換でないもの) の組から、可換なら `p q`、そうでなければ `p q p`・`q p q` を新しい対合として足す
     (`maxnnewivolus` = 100 本まで)。cod105 は生成元 11 本 + 新しい対合 44 本。
  2. 成分の置換のうち「0-1 列の上で対合で、各 2-巡回 `(i, σ(i))` が共通の set packing 制約に入る」ものが**半分以上**なら、
     それぞれを 2 列の詰め込みオービトープ (行 = 2-巡回、`i < σ(i)`、列の番号の順) にし、成分をそれだけで扱う (orbital reduction は使わない)。
  - これは各対合の lex-leader の制約 `x >=_lex σ(x)` (列の番号の順) そのもの。1 行目の右の列が 0 に固定され (41 列)、
    残りは「1 行目の右が 1 なら左が 1」型の含意をプロービングが連鎖させて固定する。
- 読み: cod105 の SCIP の決め手は SST でも symresack でもなく、この格上げ + プロービング (プロービングを切ると根は −18.2 のまま)。

## 2. 当方の側 (事実、種 1)

- 1 スレッド・60 秒 (`logs/cod105_def_t1_s1.log`・`cod105_sst_t1_s1.log`): 生成元 11 本・1 軌道 1024 列 (SCIP と同じ 11 本)。オービトープは見つからない
  (find_orbitopes は完全オービトープだけ、ORBPP は 10 列以上だけ)。既定は orbital fixing 11 本、SST は 1122 行・10 段。
  - どちらも対称性の扱いは**プロービングの後**で、根の固定は 0。SST の行 (leader と軌道の列の差) は根では何も伝播しない。
- **LP の遅さ (別の問題)**: 前処理の「LP による固定」の最初の LP が 1024 行・非零 5.7 万で 1025 反復に 31〜36 秒 (30 秒の安全網で打ち切り)、
  根の LP も 1025〜1586 反復で 26〜31 秒 (1 反復約 30 ミリ秒)。SCIP は同じ LP を 11864 反復 4.8 秒 (2500 反復/秒)。LP 担当への材料。
- fhnw-binpack4-4: 格上げの条件を満たさない (PPLEX: 0 of 1、新しい対合 44 本も詰め込みの形でない)。既定 (SST_FEAS、p2inf_20261119) で
  4 スレッド 70.0 秒・10677 ノードで実行不能 (SCIP は 8998 ノード)。ノード数は SCIP と同じ程度で、残る差は早めの FJ (実行可能性問題の上限 30 秒) と
  ノードの速さ。対称性の扱いの差ではない (推測を含む)。

## 3. 試作 `ENOMOTO_MIP_SYM_PPLEX=1` (既定は不変)

- `symmetry::packing_orbisacks` (src/mip/symmetry.rs): SCIP の 1 節 1・2 と同じ条件・同じ数 (新しい対合 `ENOMOTO_T_MIP_SYM_PPLEX_NEW` = 100、手間の上限
  `ENOMOTO_T_MIP_SYM_PPLEX_WORK` = 5e7)。詰め込みの行は「係数が全て 1、右辺 <= 1、0-1 列だけ、2 列以上」。オービトープ (完全・双子) の列を動かす生成元は除く。
- 格上げした対合は、既存の `Orbitope` (2 列、`packing`、`static_pp`、`swaps` = その対合) として ORBITOPES に入れる。既存の静的なオービトープの道に
  そのまま載る: 根で 1 行目の右の列を 0 に固定 (mod.rs の `pplex_fix`)、`ORBPP_REPROBE` (伝播・オービトープの固定・プロービングの繰り返し)、
  木では全ての行を固定の順で使う静的な orbitopal fixing、オービトープの分枝はしない。
- 格上げした成分の生成元は全て `used` にする (orbital fixing・lex-leader の行・SST から外す。分枝の順の代表と混ぜない)。ほかの成分は従来どおり。
  orbisack・SST・lex-leader の行はどれも列の番号の順の辞書式なので、別の成分で同時に使っても矛盾しない。
- ヒューリスティクスの解の並べ替え (`try_incumbent`): orbisack は列を共有するので 1 回の並べ替えで別の orbisack に反しうる。並べ替えるたびに解は
  列の番号の順の辞書式で真に大きくなるので、変わらなくなるまで繰り返す (64 回まで、PPLEX のときだけ)。それでも反すれば従来どおり
  「元の問題の解」(目的値は打ち切りに使い、答えは元の x)。暫定解は捨てない。
- サブ MIP では使わない。ログ: `MIP: symmetry: PPLEX: A of B components upgraded, C packing orbisacks (R rows) from G generators and N new involutions`。

### 3.1 効き目 (手元、種 1)

| 問題 | 設定 | 既定 | PPLEX |
|---|---|---|---|
| cod105 | 1 スレッド | 242 秒 (slowm_20261119) / 60 秒で根の LP 途中 | **37.9 秒・0 ノード** (うち 36.4 秒は対称性より前の LP による固定の最初の LP) |
| cod105 | 4 スレッド | 156.4 秒 (tree2) | **20.6 秒・0 ノード** (SCIP 1 スレッド 41.5〜64 秒) |
| physiciansched6-2 | 4 スレッド・120 秒 | 33.8 秒 | 22.3 秒 (どちらも 0 ノード、PPLEX の固定は 4 列だけなので揺れの範囲とみる) |
| glass4 | 4 スレッド・120 秒 | 時間切れ (暫定解 1.600e9) | 時間切れ (1.575e9) |

- cod105 (PPLEX、`logs/cod105_pplex_t1_s1.log`): 66 本の orbisack (17920 行) → 根の固定 19 列 → REPROBE でオービトープ 420 列・プロービング 410 列、
  自由な 0-1 列 1005 → 175、冗長な行 192 本を外す。根の LP (行 832・列 175) が 199 反復 0.07 秒で −12 (= 最適値) に届き、0 ノードで閉じる。
  SCIP (472 列・根 −13.66) より強いのは、当方の ORBPP_REPROBE がオービトープの固定とプロービングを交互に繰り返すため (推測)。

## 4. 同じ型の問題の数え上げ (`pplexscan.jsonl`、事実)

P (MIPLIB 58 問) と M (Mittelmann 186 問のうち P にない 145 問) の 203 問を、PPLEX つき・1 スレッドで前処理の後まで流した。
全て前処理の後まで届いた。対称性が見つかるのは 71 問、**PPLEX が成分を格上げするのは 6 問**:

| 組 | 問題 | 生成元 | 格上げ | REPROBE の固定 |
|---|---|---|---|---|
| P | physiciansched6-2 | 79 | 16 成分中 2、orbisack 4 本 (4 行) | 0 |
| M | cod105 | 11 | 1 成分、66 本 (17920 行) | オービトープ 420・プロービング 410 |
| M | brazil3 | 77 | 7 成分中 2、2 本 (40 行) | プロービング 18 |
| M | mzzv11 | 4 | 4 成分、4 本 (4 行) | (REPROBE なし) |
| M | glass4 | 1 | 1 本 (1 行) | 0 |
| M | nursesched-sprint02 | 1 | 1 本 (10 行) | 0 |

- 大きく効く見込みは cod105 だけ。ほかは orbisack が 1〜4 本で、orbital fixing の代わりに静的な orbisack を使う差 (良くも悪くも) を見る。
- 対称性のあるほかの 65 問は格上げの条件を満たさない (対合が少ない・2-巡回が詰め込みの行に入らない) ので、道筋は既定と同じ。

## 5. 正しさ (事実)

- cargo test (依頼の形、`--release` と python のリンク): 413 通過・0 失敗。単体テスト `packing_orbisacks_keep_a_lex_max_optimum`
  (Q3 の最大独立集合、全 256 点で orbisack と根の固定を満たす最適解が残ることを総当たりで確かめる) を含む。
- fuzz (`fuzz.sh` → `fuzz/`、全て PPLEX つき): **bad 0**。格上げした求解は計 663 回。

| fuzz | bad | 格上げした求解 / 対称性のある求解 |
|---|---|---|
| `fuzz_pplex.py 600` (既定のスレッド) | 0 / 600 | 40 / 294 |
| 同、1 スレッド | 0 / 600 | 42 / 314 |
| 同、4 スレッド | 0 / 600 | 34 / 296 |
| 同、`ENOMOTO_MIP_NO_ORBITOPE=1` | 0 / 600 | 149 / 307 |
| `mip_fuzz_symmetric.py 300 0 1` | 0 / 300 | 45 / 178 |
| 同、オービトープなし・規模 1 / 規模 2 | 0 / 300・0 / 300 | 173 / 178・180 / 215 |
| `mip_fuzz_vs_highs.py 300` | 0 / 300 | 0 / 0 |

- `fuzz_pplex.py`: Z_2^k (k = 3〜7) のケーリーグラフ (符号の型: Hamming の距離が集合に入る組を辺に) と巡回グラフの最大独立集合
  (列の番号は乱数で並べ替え、一部で重み・総数の上下限)、同じ機械への割り当て。HiGHS と状態・最適値を突き合わせ、当方の解の実行可能性と目的値も確かめる。

## 6. 広い計測 (親への依頼)

`cfg.py`: def・def_t1 (このコミットの既定)、pplex・pplex_t1 (`ENOMOTO_MIP_SYM_PPLEX=1`)。名前が `_t1` で終われば 1 スレッド、ほかは 4 スレッド。

- `shards_P.json` (MIPLIB 58 問 × 種 1〜3 × {def, pplex}、60 秒・4 スレッド、4 分担、各約 0.74 時間): `python run.py P K` (K = 0〜3)、
  集計 `python summ.py P def pplex > summ_P.txt`。働くのは physiciansched6-2 だけなので、残りは揺れの目安。
- `shards_M.json` (働く M の 5 問 = cod105・brazil3・mzzv11・glass4・nursesched-sprint02 × 種 1・2 × {def, pplex, def_t1, pplex_t1}、600 秒、
  4 分担、各 1.4〜2.1 時間): `python run.py M K` (K = 0〜3)、集計 `python summ.py M def pplex`・`python summ.py M def_t1 pplex_t1`。
- 準備は analysis/vsscip_20261117/README.md と同じ (`pip install maturin highspy==1.15.1 && pip install -e .`、このコミット以降でビルド)。作り直しは `python plan.py`。
- 採用の目安: 誤答 0・crash 0、解けた数が減らない、P の SGM の区間の上端 +5% 以内、働く問題で全種 1.25 倍超の遅れなし。

## 7. 残り・次の手

- cod105 の 1 スレッドの残り 36 秒は LP の遅さ (2 節)。対称性の扱いを LP による固定より前に置く (固定が先に入れば LP は 175 列) 案もあるが、
  前処理の順を変えるのは影響が広いので、計測の結果を見てから。
- fhnw-binpack4-4 の残りの差は対称性の外 (2 節)。

## 8. P の集計 (58 問 × 種 1〜3、60 秒・4 スレッド、4 分担そろい、`summ_P.txt`、事実)

- 348 行、crash 0、食い違い 0。解けた数 def 138 / pplex 136 (174 組)。pplex / def **+0.7% [−2.3%, +3.7%]** (どちらかが解けた 46 問)。
  全種で 1.25 倍超遅い問題はない。働く physiciansched6-2 は 0.84 (0.87・0.84・0.82、3 種とも速い)。
- pplex だけ落ちた 2 組は、どちらも PPLEX が働かない問題 (`pplexscan.jsonl`: 10teams は対称性なし、neos5 は「0 of 1 components upgraded」):
  - 10teams 種 2 (def 25.7 秒、pplex 時間切れ・暫定解 930・下界 924): 1 スレッド・種 2・60 秒 (`logs/10teams_chk_*_t1_s2.log`) で、def・pplex とも
    7048 ノードで 924 の最適 (48.3・48.6 秒)。ログの違いは時刻で出す行と LU の手間の数え (時刻による) だけ。
  - neos5 種 2 (def 40.2 秒、pplex 時間切れ・暫定解 15・下界 14.5): 1 スレッド・60 秒では両方とも時間切れ (13.7 万・13.6 万ノード、差は 60 秒で進んだ数)。
    ノードの上限 3 万でそろえると (`logs/neos5_nl_*_t1_s2.log`)、両方とも下界 13.95・暫定解 15 で、暫定解の行の並びも一致 (md5 が同じ)。
  - よって 2 組とも 4 スレッドの揺れ (PPLEX は道を変えていない)。

## 9. M の集計 (働く 5 問 × 種 1・2 × {def, pplex, def_t1, pplex_t1}、600 秒、4 分担そろい、`summ_M.txt`、事実)

- 40 行、crash 0、食い違い 0。解けた数は 4 スレッド def 6 / pplex 6、1 スレッド 6 / 6 (cod105・mzzv11・nursesched-sprint02 が全部解け、brazil3・glass4 は全部時間切れ)。
- pplex / def (どちらかが解けた 3 問): 4 スレッド **−47.5% [−83.2%, +8.8%]**、1 スレッド **−46.3% [−83.3%, −0.8%]**。全種で 1.25 倍超遅い問題なし。

| 問題 | 4 スレッド def → pplex (種 1・2) | 1 スレッド def_t1 → pplex_t1 (種 1・2) |
|---|---|---|
| cod105 | 134.7・154.8 → **21.8・25.3 秒** (17・31 → 0 ノード) | 190.2・245.0 → **32.6・37.9 秒** (83・158 → 0 ノード) |
| mzzv11 | 162.2・141.0 → 108.5・131.4 秒 | 157.1・149.9 → 156.9・147.6 秒 (ノード数 1607・645 は同じ) |
| nursesched-sprint02 | 17.2・22.4 → 20.2・22.8 秒 | 31.5・37.1 → 28.6・35.7 秒 |
| brazil3 (暫定解、小さいほど良い) | 228・243 → 207・38 | 292・89 → 304・283 |
| glass4 (暫定解 ×1e9) | 1.400・1.450 → 1.433・1.700 | 1.583・1.700 → 1.450・1.611 |
| glass4 (下界 ×1e8) | 8.234・8.238 → **8.651・8.468** | 8.000・8.000 → 8.000・8.000 |

- mzzv11 (orbisack 4 本、固定 0) は 1 スレッドでノード数が def と同じで、道筋はほぼ変わらない。4 スレッドの 0.67・0.93 は揺れの範囲とみる。
- brazil3・glass4 の暫定解は、スレッド数・種で向きが割れる (どちらも orbisack が 1〜2 本で、変わるのは 1〜数列の固定だけ)。揺れとみる。
- glass4 の下界は 4 スレッドの 2 種とも pplex が高い (+5%・+3%)。1 スレッドでは差がない (どちらも根の 8.000 のまま) ので、4 スレッドの木の進み方
  (ノード数 49 万・71 万 → 62 万・65 万) の差とみる。orbisack 1 本 (1 行) の 1 列の固定でこれだけ変わる仕組みは確かめていない (推測)。悪い材料ではない。

## 10. 採否の案

- P: +0.7% [−2.3%, +3.7%]、解けた数 138 → 136 は PPLEX の働かない 10teams・neos5 の揺れ (8 節)。M (働く 5 問): 4 スレッド −47.5%・1 スレッド −46.3%、
  全種で 1.25 倍超の遅れなし、解けた数は同じ。誤答 0 (計測の食い違い 0、fuzz bad 0・格上げした求解 663 回)、cargo test 通過。
- 採用の目安 (区間の上端 +5% 以内、解けた数を減らさない、1.25 倍超の遅れなし、誤答 0) を満たす。働くのは P・M の 203 問中 6 問と狭く、
  働かない問題では道筋が変わらない (10teams・neos5 で確かめた)。
- 提案: **`ENOMOTO_MIP_SYM_PPLEX` を既定にする** (`=0` で切る形に)。条件は SCIP と同じ「成分の置換の半分以上が詰め込みの対合」の 1 つの if で、
  効く・効かないがはっきり分かれる (cod105 は 6 倍速く、ほかはほぼ中立)。採用は利用者の判断。

## 11. 既定化 (2026-11-20、利用者の判断)

- `ENOMOTO_MIP_SYM_PPLEX` を既定にした (`=0` で以前の動き)。docs/improvement_history.md に節を足した。
- 採否の根拠 (8〜10 節): P +0.7% [−2.3%, +3.7%] (138 → 136 は PPLEX の働かない 10teams・neos5 の揺れ)、M (働く 5 問) 4 スレッド −47.5%・1 スレッド −46.3%、
  全種で 1.25 倍超の遅れなし、誤答 0、働くのは 203 問中 6 問、条件は SCIP と同じ 1 つの if。
- 既定化した版での確かめ: cargo test 414 通過・0 失敗。`fuzz_default.sh` (環境変数なし) で全部 bad 0: fuzz_pplex 600 (格上げ 35)・同 1 スレッド 600 (46)・
  mip_fuzz_symmetric 300 (50)・同オービトープなし規模 2 (194)・mip_fuzz_vs_highs 300 (`fuzz/def_*`)。cod105 は 4 スレッド 22.0 秒・0 ノード、
  `ENOMOTO_MIP_SYM_PPLEX=0` で以前どおり (120 秒で時間切れ・7 ノード、PPLEX の行なし)。
- 計測中のほかの分担 (pe の CM、cwrh0 の MCWRH0X など) は、それぞれのビルドで def と試作の対で回すので、この既定化の影響は受けない
  (この版より後でビルドし直す分担では def と試作の両方に同じく入る)。

## 12. cod105 の「LP による固定」の最初の LP が 31〜36 秒かかる原因 (親の依頼、材料、事実と推測を分ける)

- 同じ LP 緩和 (1024 行・1024 列・非零 5.7 万、行は全て詰め込み `<= 1`) の単体の比べ (1 スレッド): 当方の LP (二段解法) 10.7 秒・5486 反復
  (1.9 ミリ秒/反復、再分解が 54%、LU の非零約 17 万、26 回のうち 18 回がずれ)。HiGHS 1.15.1 (前処理なし) は双対単体法 8.1 秒・主単体法 3.3 秒・内点法 1.3 秒。
  SCIP (SoPlex) は前処理つきで 11864 反復 4.8 秒。**LP そのものは大きくは遅くない**。
- MIP の中 (1 スレッド、`ENOMOTO_MIP_SYM_PPLEX=0`、`logs/cod105_lpprof_t1_s1.log`・`cod105_lpgdb_t1.gdb`・`cod105_xodbg_t1_s1.log`): 1 スレッドの
  `TwoStageLp::solve_seq_ipm` が内点法 + クロスオーバーを先に回す。内点法は 10 反復 0.3 秒だが、**クロスオーバーの Megiddo の押し出し
  (`crossover::megiddo_push`) が 329 ピボットで 17.0 秒、うち再分解 65 回が 16.8 秒 (1 回約 260 ミリ秒)**。gdb の 10 回のスタックは全てここ
  (`megiddo_push → factorize_basis`)。その後に双対の押し出し 1.8 秒、双対単体法 1871 反復 5.6 秒で、前処理の期限 (30 秒) に当たって捨て、
  根の LP で同じことをもう一度する (35〜62 秒)。
- 再分解の引き金 (手元だけの一時的な診断の行で数えた、コミットしていない): 76 回とも `RefactorDue::Fill` で、FT 更新 5 回目の周期検査。
  クロスオーバーの `BasisKernel::new` の eta の fill の上限は `FT_BUMP_LIMIT_FACTOR · m` = 64 × 1024 = 65536 のまま。双対単体法は
  `with_periodic_check(…, FT_BUMP_LU_RATIO = 3)` で上限を `3 · nnz(LU)` (この基底では約 90 万) に広げているが、クロスオーバーでは広げていない。
  この問題の基底はほぼ密 (入る列の FTRAN の非零 1024 = m) なので、更新 1 回で 1 万を超える fill が出て、5 回ごとに必ず上限を越える。
- 確かめ (既存の調整値だけ、全体に効くので直しではない): `ENOMOTO_T_FT_BUMP_LIMIT_FACTOR=1024` で Megiddo の押し出しは **17.0 → 0.5 秒**
  (再分解 1 回)、最初の LP は 7.1 秒で解け (以前は 30 秒の期限で打ち切り)、根の LP は 8.3 秒 (`logs/cod105_bump1k_t1_s1.log`)。
- 直しの案 (未着手): クロスオーバーの基底の核 (Megiddo の主・双対の押し出し) にも双対単体法と同じ `FT_BUMP_LU_RATIO · nnz(LU)` の上限を使う。
  影響はクロスオーバーを通る LP (主に 1 スレッドの根・前処理の LP) だけで、密な基底でしか上限が変わらない (疎な基底では `64 m` の方が大きい)。
  残りの 7 秒のうち 5.6 秒は双対単体法のずれによる再分解 (62%) で、これは LP の担当の領分。PPLEX の既定化で cod105 自体はこの LP の前に
  終わる形ではない (LP による固定は対称性より前) ので、cod105 の 1 スレッド 32〜38 秒の大半はこの 2 つ。

## 13. 試作 `ENOMOTO_XO_BUMP_LU_RATIO=1` (クロスオーバーの押し出しの fill の上限、既定は不変、親の依頼)

- 中身: `crossover::xo_kernel` (src/simplex/crossover.rs)。Megiddo の主の押し出し (`megiddo_push`) と双対の押し出し (`megiddo_dual_push`) の基底の核
  (`BasisKernel::new` の 6 か所) に、双対単体法と同じ fill の上限 `FT_BUMP_LU_RATIO · nnz(LU)` を付ける (`BasisKernel::with_bump_lu_ratio` を足した。
  値は双対単体法の `ENOMOTO_T_FT_BUMP_LU_RATIO` (既定 3) を共有、新しい定数は足さない)。広がるのは `nnz(LU) >= 64 m` の密な分解だけ (`fill_too_big`)。
- cod105 (1 スレッド・種 1、`logs/cod105_xo{0,1}_t1_s1.log`・`cod105_ppxo{0,1}_t1_s1.log`):
  - PPLEX を切った形: 押し出し 14.6 秒 (再分解 65 回) → **0.48 秒 (1 回)**。前処理の LP による固定の最初の LP は 30 秒の期限で打ち切り → 7.6 秒で解け、
    根の LP は 69.8 秒 → **6.9 秒**。
  - 既定 (PPLEX あり): 33.9 秒 → **25.7 秒** (どちらも 0 ノードで最適)。
- 走査 (`xoscan.py` → `xoscan.jsonl`・`xoscan_summ.txt`、既定、1 スレッド・60 秒・`ENOMOTO_DEBUG_CROSSOVER=1`、P・M の 203 問、エラー 0):
  根までに Megiddo の押し出しを通るのは 144 問、基底の核の引き金 (照合・x_B の残差以外) の再分解があるのは 58 問。ただし押し出しの再分解と
  双対の押し出しの時間が 0.5 秒以上なのは cod105 (押し出し 20.2 秒・再分解 65 回)・neos-1354092 (再分解 1.25 秒)・chromaticindex512-7 (0.57 秒 + 双対 0.98 秒) の 3 問、
  0.1 秒以上でも 7 問 (ほかに radiationm40-10-02・neos-4300652-rahue・mzzv11・dano3_5)。
  - XO で変わるか (1 スレッド・60 秒): chromaticindex512-7 は再分解 11 回のまま (変化なし)、neos-1354092 は 44・42 回 → 44・40 回。どちらも分解が
    `64 m` に届かず上限が変わらない。**XO がはっきり効くのは cod105 だけ** (密な基底)。
- 正しさ: cargo test 414 通過。fuzz (`fuzz_xo.sh`・`fuzz/xo_*`) は全部 bad 0: mip_fuzz_vs_highs 300 (4 スレッド・1 スレッド)、lp_fuzz_vs_highs 300
  (二段解法・ipm_crossover)、fuzz_pplex 300 (1 スレッド)、mip_fuzz_symmetric 300。
- LP のベンチ (`scripts/crossover_bench/run.py --set netlib --variants base= xo=ENOMOTO_XO_BUMP_LU_RATIO=1`、ipm_crossover、`xobench_netlib.jsonl`・
  `xobench_netlib_summ.txt`): 93 問とも両方解け、状態の違いなし、SGM (shift 1 秒) xo / base 0.997。Netlib は `nnz(LU)` が 64 m に届かないので道は同じ (揺れだけ)。
- 記録だけ (親の指示): cod105 の残りの LP の時間の大半は双対単体法のずれによる再分解 (1871 反復で 5.6 秒のうち再分解 62%、`refactor_count=10`
  が全部 drift)。LP の担当の領分。

### 13.1 計測の依頼 (cfg.py: def・xo・def_t1・xo_t1)

- `shards_PXO.json` (MIPLIB 58 問 × 種 1〜3 × {def, xo, def_t1, xo_t1}、60 秒、8 分担、各約 0.84 時間): `python run.py PXO K` (K = 0〜7)、
  集計 `python summ.py PXO def xo`・`python summ.py PXO def_t1 xo_t1`。P で押し出しの再分解が 0.1 秒を超える問題はない (走査) ので、揺れの目安。
- `shards_MXO.json` (M の 7 問 = cod105・neos-1354092・chromaticindex512-7・radiationm40-10-02・neos-4300652-rahue・mzzv11・dano3_5 × 種 1・2 ×
  {def, xo, def_t1, xo_t1}、600 秒、5 分担、各 1.4〜2.1 時間): `python run.py MXO K` (K = 0〜4)、集計 `python summ.py MXO def xo`・`python summ.py MXO def_t1 xo_t1`。
- 作り直しは `python plan_xo.py`。この版 (XO の試作のコミット) 以降でビルド。

### 13.2 PXO の 10teams (4 スレッドで 3 種とも xo が遅い、親の知らせ) の切り分け (事実)

- 計測機: 種 1 5.7 → 13.4 秒、種 2 12.8 → 25.6 秒、種 3 7.4 → 18.5 秒 (1 スレッドは差なし)。
- XO が働いているか:
  - 1 スレッド・種 2・ノードの上限 3000 で、def と xo のログは時刻の入る行と最後の結果の時間を除いて全行一致 (`logs/10teams_nlxo{0,1}_t1_s2.log`)。
  - 4 スレッド・種 1〜3 (`logs/10teams_txo{0,1}_t4_s*.log`、`ENOMOTO_DEBUG_CROSSOVER=1`): クロスオーバーの押し出しは 1 回の求解で 2〜3 回、
    再分解は毎回 2 回で、def と xo で同じ。押し出しの回数の差 (種 1 で 2 回と 3 回) は 4 スレッドの根の競争の時刻の違い。
  - 10teams の基底の分解は `64 m` に届かず (走査で押し出しの再分解 0.00 秒)、XO の上限は変わらない。**XO は 10teams の道を変えていない**。
- 手元の 4 スレッド・60 秒 (`rep10teams.txt`、種 1〜6、秒): def 5.0・16.0・7.6・14.7・56.5・6.8、xo 4.9・26.8・13.0・31.4・9.4・60.0 (時間切れ)。
  その前の回 (種 1〜3) は def 9.6・16.1・48.4、xo 5.1・18.7・24.5。同じ種・同じ設定でも 5〜57 秒に散り、向きも割れる。
- 原因 (推測を含む): 10teams の 4 スレッドは、根の下界 924 のまま「どのスレッドが 924 の解を見つけるか」で決まる裾の重い問題 (p2inf_20261119 5.8 節と同じ形)。
  計測機の 3 種とも xo が遅かったのはこの揺れ (3 種そろって同じ向きになる確率は約 1/8)。1 問対策の直しは要らないと判断。
- 提案: 採用の目安「全種で 1.25 倍超」の判定から 10teams を外すのではなく、PXO の 10teams だけ種 4〜9 を def・xo で追加して見る (各 60 秒 × 12 回、約 15 分)。
