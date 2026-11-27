# T1PRIMAL: 1 スレッドの解探し (2026-11-26〜27、親の依頼、gap240 の型 1)

gap240_20261126 の型 1 (1 スレッドの解探し、効き 0.972) の試作。ENOMOTO_MIP_T1PRIMAL=1 で入る (既定は切)。
計測の版は e110c4660 の .so (ALNS 既定化 cd676b114 より前)。次の計測は 82071c738 以降で行う。

## 1. 作り (src/mip/t1primal.rs)

- 4 スレッドの FJ スレッドが回す FJ + Local-MIP の組 (fj_lmip_mix) を、再開できる形 (Mix) に写し、1 スレッドでは別スレッド
  「t1primal」に置いて、主スレッドが時間の切れ端 (Slice) を渡したときだけ回す (主スレッドはその間待つので、使うのは 1 コア)。
- 入れる所: 根 (T1MIX の後、根が閉じていないとき) と、木の新しい潜り (plunge_depth == 0) の頭。
  条件は mip_threads()==1・submip でない・再スタートの前・proof_only でない・FJ スレッドがない。
- 手間の取り分 s = 0.25·σ(logit h + logit f + logit g)。新しい閾値は入れていない。
  - h = (当たり+1)/(切れ端+2) (当たりの率、ラプラス)
  - f = (今 − 最後の改善)/今 (改善が止まっている割合)
  - g = 根の下界からの相対ギャップ (暫定解なしは 0.5)
  - 0.25 は 4 スレッドの heur_role の取り分 (ENOMOTO_T_MIP_HEUR_ROLE_FRAC) と同じ。
- 預金 = s·今 − 使った時間 が切れ端の最小 (FJ_MIX_SLICE、0.2 秒) を超えたら、min(預金, 残り時間の半分) を回す。
- 返った解は新しい順に試し、ラベルは「T1PRIMAL」。
- ALNS (FVbj) とは別勘定 (ALNS の frac には触れない)。予算をまとめるかは両方の P の結果が出てから相談する。

確かめ: fuzz.sh (ファザー 7 つを 1 スレッド・T1PRIMAL=1・切れ端 0.002 秒で回す) で不良 0、cargo test 425 通過 (fuzz.out)。

## 2. 狙いの 4 問 (L、1 スレッド、600 秒、種 1〜3、summ_L.txt)

比 0.784 [0.543, 1.133]、解けた数 5 → 8、食い違い 0。

| 問題 | 種 | 既定: 最初の解 / 全体 | T1PRIMAL: 最初の解 / 全体 |
|---|---|---|---|
| 30n20b8 | 1 | 261 / 335 | 15 / 129 |
| 30n20b8 | 2 | 268 / TL (453) | 16 / 123 |
| 30n20b8 | 3 | 255 / 458 | 24 / 523 |
| CMS750_4 | 1 | 0.3 / TL (525) | 0.3 / TL (259) |
| CMS750_4 | 2 | 0.3 / TL (484) | 0.3 / 544 |
| CMS750_4 | 3 | 0.3 / TL (491) | 0.4 / 301 |
| bnatt400 | 1 | 482 / TL | なし / TL |
| bnatt400 | 2 | なし / TL | なし / TL |
| bnatt400 | 3 | なし / TL | なし / TL |
| neos-1171448 | 1 | 4.2 / 224 | 3.8 / 275 |
| neos-1171448 | 2 | 3.9 / 204 | 4.7 / 203 |
| neos-1171448 | 3 | 6.0 / 35 | 6.3 / 34 |

- 30n20b8・CMS750_4 には FJ + Local-MIP の組がよく効く (4 スレッドで先に解が出る理由と同じ。logs/diag_30n20b8_t4・_t1p)。
- bnatt400 の解は木の LP から出る (SCIP も relaxation、ノード 467)。T1PRIMAL の取り分だけ木が遅れ、種 1 で既定の解を落とした。
- neos-1171448 は変わらない。

## 3. P (1 スレッド、60 秒、種 1〜3、58 問、summ_P.txt)

比 1.006 [0.997, 1.014]、解けた数 116 → 116、食い違い 0。全種 1.25 倍超の遅れ: markshare_4_0 [1.54, 1.37, 1.37]。
ほかに qiu 1.11、noswot 1.08、nw04 1.08 (根で閉じる問題)。T1MIX (+6.5%) よりは小さいが、目安からは外れ。

原因 (logs/diag_markshare_4_0_tp・_def、diag_qiu_tp・_def):
- markshare_4_0 は暫定解 1・下界 0 で g = 1.0 のまま。logit g = +∞ が当たり 0/16 (h 0.06) を押し流し、取り分が 0.25 に張り付く。
  既定との差 3.5 秒は T1PRIMAL が使った 3.2 秒とほぼ同じ (ノード数は同じ 272319)。qiu も g 0.95 で同じ形。
- 暫定解なしのときは f = 1 (logit +∞) で、h がいくら下がっても 0.25 のまま (bnatt400 の損も同じ所から)。

直し方の案 (次の版): f と g も h と同じ数 (切れ端の数 n) でラプラスにならし (x̃ = (x·n+1)/(n+2))、外れが続けば
h の下がりが f・g の上がりと打ち消し合って取り分が下がるようにする。暫定解なしの g は 1 にそろえる。

## 4. 4 スレッド

T1PRIMAL は mip_threads()==1 でだけ入る。ENOMOTO_MIP_T1PRIMAL=1・4 スレッドで 30n20b8・markshare_4_0 を回し、T1PRIMAL の行が
0 であることを確かめた (logs/diag_*_tp_t4)。4 スレッドの道は変わらない。

## 5. SCIP の潜りの統計 (当方にない潜りの候補)

- 30n20b8: 最初の解は linesearchdiving (88 秒、ノード 339)。ほかに conflict・objpscost・pscost diving、alns。
- bnatt400: 最初の解は relaxation (ノード 467)。
- CMS750_4: locks、feaspump、objpscostdiving。
- 当方にないもの: linesearch・distribution・adaptive・rootsol diving。第一候補は linesearch diving (30n20b8 の最初の解)。

## 6. T1MIX の教訓

T1MIX は固定の取り分で FJ・Local-MIP を 1 スレッドで回し、P の 1 スレッドで +6.5%。今回は暫定解なし・改善の停滞・当たりの率で
取り分を絞る形にしたので +0.6% まで下がったが、g の飽和で絞りが効かない問題 (markshare_4_0) が残った (3 節)。

## ファイル

cfg.py・run.py・plan.py・summ.py (unify の写し)、shards_L.json・shards_P.json、runall.sh、res_L_s0.jsonl・res_P_s0.jsonl、
summ_L.txt・summ_P.txt、fuzz.sh・fuzz.out、logs/*.gz (診断)。
