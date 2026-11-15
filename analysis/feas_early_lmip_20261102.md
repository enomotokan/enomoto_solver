# 実行可能性問題の早めの探索に Local-MIP を並べる試作 ENOMOTO_MIP_FEAS_FJ_EARLY=fl (2026-11-02)

- 環境: クラウドコンテナ、CPU 4。ソルバーは同時に 1 つだけ。4 スレッド・60 秒、`ENOMOTO_MIP_LOG=1`。
  **この日の機械は前回 (analysis/ns1116954_20261101.md) より 1.7 倍ほど遅い** (ns1116954 の既定が 30 秒 → 50〜57 秒、
  前処理の LP による固定が 11 秒 → 23 秒)。時間は同じ日の形どうしで比べる。
- コミット: e1a3eb94 (bench13 の fjearly の形 + 試作 `=fl`。他のセッションの 9dd449d4 以降の変更は入っていない wheel で計測)。既定・`=1` の道は変えていない (`=fl` のときだけ新しい関数を通る)。
- 台本・ログ: `analysis/feas_early_lmip_20261102/` (`objcheck.py` 目的関数の係数の数え、`objcheck.out`、`run_eno.py`、`run.sh`
  (4 形 × 3 回を交互に)、`summarize.py` → `table.md`、`results.jsonl`、`logs/<問題>_<形>_r<回>.log`、`logs/survey_*` 下見、
  `logs/smoke*` ns1116954 の試し)。
- 計測の事故: 計測中に途中の結果をコミットして `git rebase` したため、その間に追記された `results.jsonl` の 6 行が失われた
  (前回の記録と同じ事故)。その 6 回 (krka fj r1、binpack fj r1・r2、ns1952667 fl31 r1、pythago def r2、neos-578379 def r2) は
  計測の後で測り直した (ログも測り直しのもの)。**計測中は rebase しないこと**。

## 1. 結論

- 試作 `ENOMOTO_MIP_FEAS_FJ_EARLY=fl`: `=1` と同じ場所 (前処理後・LP による固定の前、目的関数が全部 0 のとき) で、スレッドの半分を
  種を変えた FJ2、残りを種を変えた Local-MIP (連鎖の手あり) にして並べて回す。どれかが見つけたら全部止めて最適で終える。
- 目的関数が全部 0 の 9 問 (各 3 回、表は 3 節):
  - **Local-MIP が FJ の苦手な問題を解く**: fhnw-binpack4-48 は既定・`=1` で 0/3 (60 秒で解なし) → `=fl` で 3/3 (1.5〜1.6 秒)。
    neos-3004026-krka は既定 17〜18 秒・`=1` 23 秒 (早めの FJ が 6 秒で見つけられず損) → `=fl` 1.2〜1.3 秒。lectsched-1 は
    `=fl` の 1 回だけ Local-MIP が 5.94 秒で見つけた (ほかはすべて解なし)。
  - **ただし FJ を半分にすると ns1116954 を失う**: `=1` (FJ 4 本) は 4.6〜4.8 秒で見つけ 5.1〜5.5 秒で最適だが、`=fl` (FJ 2 本) は
    6 秒の上限までに見つからず 3 回とも既定の道 (58〜60 秒、既定 50〜57 秒より 6 秒の損) に戻った。FJ の 1 回が 1.7 秒ほどかかり、
    当たりの種が 9〜10 回目あたりなので、2 本では間に合わない。Local-MIP はこの問題では違反 1 行で止まる。
  - **FJ 3 本 + Local-MIP 1 本 (`ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS=1`、表では fl31) が一番よい**: ns1116954 5.5〜6.2 秒
    (`=1` とほぼ同じ、上限ぎりぎりではある)、krka 1.4 秒、binpack 1.5〜1.6 秒。lectsched-1 は 0/3。
  - 残りの 5 問 (cryptanalysiskb128n5obj16・ns1952667・pythago7824・neos-578379・supportcase3) はどの形でも 60 秒で解なし。
    見つからないときの損は早めの探索の 6 秒 (0.1 × 時間上限) で、木のノードが 1〜2 割減る程度 (ns1952667 既定 2440〜2867 →
    fl31 2032〜2100)。解けた数は変わらない。
- 提案: 58 問・Mittelmann で確かめるなら、**`ENOMOTO_MIP_FEAS_FJ_EARLY=fl` + `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS=1`** (fl31) を
  `=1` と比べる (5 節)。

## 2. 仕組み (`src/mip/mod.rs` の `feas_fj_lmip_early`)

- 呼ぶ場所・条件は `=1` の `feas_fj_early` と同じ: `presolve_mip_depth` の LP による固定 (`lp_probe`) の直前、最初の求解 (サブ MIP・
  再スタートでない) で前処理後の目的関数が全部 0 (`heuristics::fj_feas_extend`)。問題も同じ (前処理後の問題。FJ_P3 の前処理は
  かけない。FJ のスレッドの FJ_P3 は ns1116954 で 3〜4.5 秒かかって何も減らさないので、早めの探索には入れない)。
  見つけた後の扱い (`FEAS_FJ_SOL`、クリーク行の拡張・対称性・木を飛ばして最適) も `=1` と共通。
- スレッド: `threads` 本のうち Local-MIP は `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS` 本 (既定は半分の切り捨て、4 本なら 2 本)、
  残りが FJ2。FJ2 の種は `=1` と同じ並び (FJ のスレッドの数で刻む)。Local-MIP は 1 本ずつ別の種で、上限まで 1 本の探索を続ける
  (中でやり直しをする)。スレッドが 1 本なら 1 本の中で FJ2 を 1 回と Local-MIP の 1 切れ (`ENOMOTO_T_MIP_FEAS_EARLY_LMIP_SLICE`、
  0.2 秒、続きから) を交互に回す。
- 時間の上限・止め: 上限は `cap` (`ENOMOTO_T_MIP_FEAS_FJ_EARLY_FRAC` (0.1) × 時間上限) と前処理の期限 (`PRESOLVE_DEADLINE`) の
  早い方を最初に決めて全スレッドに渡す (期限はスレッド局所なので主で読む)。どれかが見つけたら止めの合図を立てる。
  - FJ2 は止めの合図を見る。止めの合図が立った後は LP 修復をしない (`fj::SKIP_REPAIR_ON_STOP`、FJ と Local-MIP を混ぜる
    `heuristics::fj_lmip_mix` と同じ。`=1` はこれを立てていない)。
  - Local-MIP (`LocalMip::run`) は 16 手ごとに期限と止めを見る (連鎖の手の中でも見る)。初期化 (`LocalMip::new`、左辺の計算・
    集合の行の表) は止めを見ないので、始める前に一度見る。初期化は 9 問とも 0.06 秒以下 (ns1116954 の 13 万行でも)。
  - 9 問・108 回で早めの探索の終わりは最大 6.02 秒 (`=1` も 6.01 秒、上限 6 秒)。上限の超過はない。
- ログ: `MIP: presolve: early FJ/Local-MIP for the feasibility problem: 4 threads (FJ 2, Local-MIP 2), FJ k tries, Local-MIP s steps,
  found true by Local-MIP (t s, cap 6.00s)`。Local-MIP のスレッドごとに初期化の秒数と、手の数・違反行の最少・連鎖の手の数
  (`MIP:   early Local-MIP thread ...`)。違反行の最少は見つけたスレッドでも 0 にならない (見つけた手で抜けるので数え直さない)。
- 診断用: `ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN=1` で連鎖の手を切る。ns1116954 では連鎖ありで 2 本 2176 手、連鎖なしで
  43.6 万手 (1 手が 200 倍軽い) だが、どちらも違反 1 行で止まり見つからない (`logs/smoke2_*`)。

## 3. 結果 (`table.md`)

解けた回数と時間 (秒、切 = 60 秒で解なし)。def = 既定、fj = `=1`、fl = `=fl` (FJ 2 + Local-MIP 2)、fl31 = `=fl` で FJ 3 + Local-MIP 1。

| 問題 | 行・列 | def | fj | fl | fl31 |
|---|---|---|---|---|---|
| ns1116954 | 131991・12648 | 3/3: 56.5 / 53.3 / 50.4 | 3/3: 5.5 / 5.1 / 5.3 | 3/3: 59.9 / 57.7 / 58.7 | 3/3: 6.0 / 6.2 / 5.5 |
| neos-3004026-krka | 12545・17030 | 3/3: 17.7 / 17.8 / 17.4 | 3/3: 23.1 / 23.1 / 23.4 | 3/3: 1.2 / 1.3 / 1.2 | 3/3: 1.4 / 1.4 / 1.4 |
| fhnw-binpack4-48 | 4480・3710 | 0/3 | 0/3 | **3/3: 1.5 / 1.6 / 1.6** | **3/3: 1.5 / 1.6 / 1.6** |
| cryptanalysiskb128n5obj16 | 98021・48950 | 0/3 | 0/3 | 0/3 | 0/3 |
| ns1952667 | 41・13264 | 0/3 | 0/3 | 0/3 | 0/3 |
| pythago7824 | 14652・3740 | 0/3 | 0/3 | 0/3 | 0/3 |
| lectsched-1 | 50108・28718 | 0/3 | 0/3 | 1/3: 6.2 | 0/3 |
| neos-578379 | 21703・17010 | 0/3 | 0/3 | 0/3 | 0/3 |
| supportcase3 | 12702・4191 | 0/3 | 0/3 | 0/3 | 0/3 |

早めの探索で見つけた方と時刻 (秒、既定は木の中の最初の暫定解とその出どころ):

| 問題 | def | fj | fl | fl31 |
|---|---|---|---|---|
| ns1116954 | FJ のスレッド 19.1〜21.9 | FJ 4.6〜4.8 | なし ×3 | FJ 4.9〜5.5 |
| neos-3004026-krka | 根の local search 16.4〜16.9 | なし ×3 | Local-MIP 1.18〜1.22 | Local-MIP 1.32〜1.35 |
| fhnw-binpack4-48 | なし | なし ×3 | Local-MIP 1.50〜1.63 | Local-MIP 1.48〜1.55 |
| lectsched-1 | なし | なし ×3 | なし ×2、Local-MIP 5.94 | なし ×3 |
| 残り 5 問 | なし | なし | なし | なし |

- krka は既定では根のヒューリスティクスの local search が 16〜17 秒で見つける (前処理の LP などの後)。`=1` は早めの FJ が 6 秒
  見つけられずに既定の道に戻るので、既定より 6 秒遅い (23 秒)。
- Local-MIP が見つけられなかった問題の違反行の最少 (`fl` の 1 回目): ns1116954 1、lectsched-1 1、neos-578379 60〜67、
  supportcase3 220〜238、ns1952667 33〜34、pythago7824 9〜17、cryptanalysis 38〜45。lectsched-1 は惜しい (1 回は解けた)。

見つからないときの損 (時間切れの回の木のノード数):

| 問題 | def | fj | fl | fl31 |
|---|---|---|---|---|
| ns1952667 | 2575 / 2867 / 2440 | 1964 / 2165 / 2192 | 1666 / 1998 / 2143 | 2057 / 2100 / 2032 |
| lectsched-1 | 1066 / 1102 / 1079 | 892 / 1067 / 778 | 992 / 822 (1 回は解けた) | 904 / 959 / 1010 |
| supportcase3 | 326 / 327 / 332 | 288 / 259 / 299 | 282 / 259 / 317 | 261 / 296 / 305 |
| crypt・pythago・neos-578379 | 0 | 0 | 0 | 0 |

## 4. 問題の選び方

- 58 問 (analysis/cbcimpl_20261029/probs.py) で最適値 0 の問題は ns1116954 だけ。
- `scripts/miplib2017-v31.solu` で値 0 の問題 43 問のうち、Mittelmann の組 (analysis/cbcimpl_20261029/mitt_parts.json) にある 7 問と、
  ほかの 12 問を取得して MPS の目的関数の行 (最初の N 行) の 0 でない係数を数えた (`objcheck.py` → `objcheck.out`)。
- 0 だったのは neos-3004026-krka・ns1116954・fhnw-binpack4-48・cryptanalysiskb128n5obj16・ns1952667 (以上 Mittelmann)・pythago7824・
  lectsched-1・neos-578379・supportcase3 の 9 問。これを全部使った。academictimetablesmall・cbs-cta は最適値 0 だが係数がある。
- acc-tight4 (係数 1 個) は前処理後も目的関数が残り、早めの探索が動かない (`logs/survey_acc-tight4_fl.log`) ので外した。

## 5. 58 問・Mittelmann で確かめる形の提案

- 形: **`ENOMOTO_MIP_FEAS_FJ_EARLY=fl ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS=1`** を `ENOMOTO_MIP_FEAS_FJ_EARLY=1` (bench13 の
  fjearly) と比べる。動くのは前処理後の目的関数が全部 0 の問題だけなので、58 問では ns1116954 だけが変わるはず (FJ 3 本で 5.5〜6.2 秒、
  上限 6 秒の近くなので揺れを見る)。Mittelmann では krka・fhnw-binpack4-48 (+1 問) が良くなり、ns1952667・cryptanalysiskb128n5obj16
  は変わらないはず。
- 既定にするなら FJ 3 本・Local-MIP 1 本 (4 スレッドのとき) を既定の割り振りにする (今の `=fl` の既定は半分)。ns1116954 が上限に
  近いので、上限を 0.1 → 0.15 × 時間上限に上げるか、Local-MIP が違反行を減らせなくなったら FJ に回す形も候補 (未計測)。
- 見つからない問題の損は 6 秒 (時間上限の 1 割) で、`=1` と同じ。

## 追記: 既定の比を FJ 3 対 Local-MIP 1 に (2026-11-02)

- `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS` の既定を「半分の切り捨て」から「4 分の 1 の切り捨てで少なくとも 1 本」に変えた
  (`src/mip/mod.rs` `feas_fj_lmip_early`)。スレッドの数ごとの分け方 (FJ + Local-MIP): 2 本 → 1 + 1、3 本 → 2 + 1、
  4 本 → 3 + 1、8 本 → 6 + 2、16 本 → 12 + 4。1 本なら今まで通り交互。
- 4 本では fl31 (`=1`) と同じなので、bench14 の flbgx (`LMIP_THREADS=1` を明示) の結果はそのまま当てはまる。
- `ENOMOTO_MIP_FEAS_FJ_EARLY` 自体はまだ既定では使わない (bench14 の結果を見て決める)。
