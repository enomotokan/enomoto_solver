# 群 2: 木の効率で SCIP より遅い問題 (2026-11-18、BRIEF.md の群 2)

対象: cod105・istanbul-no-cutoff・fast0507・dano3_3・dano3_5・supportcase42・seymour (BRIEF)、親の指示で neos-1445765 (vsscip M 49)・
rail507 (vsscip M 47) を足した 9 問。機械は 4 CPU のクラウドのコンテナ。

## 台本

- `scipab.py 問題 設定[,設定...] [--tl 秒]`: SCIP 10.0.2 (pyscipopt 6.3.0) の機能を 1 つずつ切って解き、`scipab.jsonl` に足す。
  設定の名前は `apply` を見る (`p:名前=値;...` で任意の SCIP のパラメータ)。`jobs.txt`・`jobs2.txt`・`jobs3.txt` が回した並び
  (SCIP は 1 スレッドなので 4 本並べて回した。時間は 4 本並べたときのもの)。
- `ours.py 問題 名前 [--tl 秒] [--threads N] [--seed N] [--log] -- 環境変数=値 ...`: 当方を 1 回解き、`ours.jsonl` に足す
  (ログは `logs/`、追跡しない)。`chainA.sh`・`chainB.sh` が回した並び。ソルバーは同時に 1 つ、4 スレッド。

## 1. SCIP の決め手の切り分け (scipab.jsonl、種 1)

既定と、1 つだけ切ったときの時間 (秒、TL = 時間切れ) と根の下界。当方は vsscip_20261117 の def (4 スレッド)。

| 問題 | SCIP 既定 | 決め手 (切ると) | ほか | 当方 def |
|---|---|---|---|---|
| cod105 | 44.3、根 −13.53 | usesymmetry=0 で TL・根 −18.29。ビット 1 (対称性の制約) だけで 48.3・−13.53、ビット 4 (SST) だけで 54.9・−13.49、ビット 2 (オービタルの簡約) だけで 333・−18.29 | 双対の簡約を切っても TL (対称性の制約が使えない)。分離を切ると 9.8 秒 | 144.9、根 −18.29 |
| neos-1445765 | 46.0 | 前処理を全部切ると TL。前処理器・制約の dualpresolving を 1 つずつ切ってもどれも 45〜62 | — | 109.0 |
| rail507 | 216.2 | presolving/domcol を切ると TL (176)。allowstrongdualreds=0 も TL | dualfix を切ると 592。ほかの前処理器は 221〜237 | TL (175・下界 173) |
| fast0507 | 328.9 | 前処理・双対の簡約を切ると TL | 分離・ヒューリスティクスを切ると速い (181・162) | TL (175・173) |
| istanbul-no-cutoff | 100.6、根 129.4 | separating/aggregation を切ると根 76.6・175 秒、cmir を切ると 87.7・160 秒 | 分離を全部切ると根 66.8・154 秒。ほかの分離器は小さい | 298.9、根 70.6 |
| seymour (300 秒) | 下界 415.66、根 411.05 | zerohalf を切ると根 408.79・下界 413.47 | 分離を全部切ると根 405.3。ほかの分離器は小さい | 600 秒で 412.0、根 408.5 |
| supportcase42 (300 秒) | 下界 7.751、12304 ノード・LP 反復 52711 | どれを切っても 7.751 (前処理なしで 7.711) | — | 600 秒で 7.344・920 ノード |
| dano3_3 | 87.1 | 決め手なし (68〜101) | ノード数は当方と同じくらい (14 対 7〜15) | 224.2 (t1 186.1) |
| dano3_5 | 239.6 | 決め手なし (pscost で TL、ほかは 257〜410) | — | 455.6 (t1 303.4) |

SCIP の rail507 の前処理 (1 ノードで止めて統計): domcol が 63019 列のうち **42278 列を固定** (0.64 秒、4 回)、dualfix 18 列、
前処理後 20698 列・441 行。

## 2. 当方の側 (ours.jsonl、4 スレッド・600 秒・種 1)

- cod105: 既定 156.4 秒 (根 −18.29)、`ENOMOTO_MIP_ORBPP=1` 148.4 秒 (根 −18.29、変わらず)、`ENOMOTO_MIP_SYM_SST=1` 258.1 秒・8773 ノード
  (根 −16.16 に上がるが遅い)。当方の対称性の検出は生成元 10〜11 個で「上限で止まる」(SYMPRE probe)。SCIP の対称性の制約 (symresack・
  オービトープ) に当たるものが当方にはない。
- rail507: 既定 TL (175)。`ENOMOTO_MIP_DOMCOL=1` は 1 列、`+ _PRED=1` は 26 列だけ固定して TL。組の上限 (`ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS`
  など) を 10 億にしても 9428 列・13 秒。

### 原因 (rail507・fast0507)

当方の支配される列の固定 (`dominated_binary_fixings_with`) の (c) は、支配する側 y_j の最も短い「締める」行から y_k を探す。集合被覆
(Σ x ≥ 1 の行だけ) では「y_j が y_k を支配する」は「y_j の列が y_k の列を含み、費用が安い」で、y_k は y_j の最も短い行にあるとは限らないので、
候補がほとんど出ない。SCIP の domcol は支配される側から探すので漏れない。

### 試作 `ENOMOTO_MIP_DOMCOL_REV` (既定は不変、DOMCOL・DOMCOL_PRED と併せて使う)

支配される側 y_k の「行を助ける向き」(下限側で係数が正、上限側で負) の最も短い行から y_j を探す (y_j が y_k を支配するなら、その行には
y_j も入る)。固定の条件は (c) と同じ (worst_case_ok か y_j の上限 1 が行から導ける)。組の上限 `ENOMOTO_T_MIP_DOMCOL_REV_MAX_PAIRS` (既定 5000 万)。

- rail507 (種 1): 36886 列を 0.50 秒で固定 (前処理後 26114 列・非零 466275 → 192844)、**258.6 秒で最適 174** (既定は TL)。
- rail507・fast0507 の種 1〜3 (4 スレッド・600 秒、秒、TL = 時間切れ):

| 問題 | 種 | def | drev |
|---|---|---|---|
| rail507 | 1 | TL (175) | **258.6** |
| rail507 | 2 | 365.2 | **177.8** |
| rail507 | 3 | 347.8 | **242.3** |
| fast0507 | 1 | TL (175) | **239.6** |
| fast0507 | 2 | TL (175) | **207.5** |
| fast0507 | 3 | TL (176) | **299.4** |

  fast0507 でも 36886 列を 0.35 秒で固定。6 組すべて最適 174 (SCIP は rail507 216 秒・fast0507 329 秒)。drev = DOMCOL + DOMCOL_PRED + DOMCOL_REV。
- 誤答の確かめ: `fuzz_drev.sh` (release019 の fuzz の組を drev で、7 組・計 5100 問) で誤答 0。`fuzz_cover.py` (支配の組を多く作る被覆型の
  0-1 問題、HiGHS と突き合わせ) で 2300 問・誤答 0 (300 問で固定の数は REV なし 170 → あり 406)。`cargo test --release --lib` 415 通過・0 失敗。

## 3. 広い計測 (domrev/)

`domrev/` は slow2_20261117 の台本の写し (設定は `cfg.py` の def・drev)。
- M = Mittelmann 48 問 (vshighs の M) + rail507・fast0507 × 種 1・2 × {def, drev}、600 秒、10 分担 (各 1.7 時間): `cd analysis/vsscip_slow_20261118/domrev && python run.py M K` (K = 0〜9)
- P = MIPLIB の 58 問 × 種 1〜3 × {def, drev}、60 秒、4 分担 (各 0.74 時間): `python run.py P K` (K = 0〜3)
- 集計: `python summ.py M drev`・`python summ.py P drev`。
