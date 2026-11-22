# RENS の近傍にダイブを足す (2026-11-22、利用者の案)

案: RENS のサブ MIP を作らず、主の LP のまま RENS と同じ固定 (LP で整数値の列を固定、他を ⌊x̄⌋〜⌈x̄⌉) を境界として置き、伝播してから
根のダイブをその定義域で回す (サブ MIP の前処理・立ち上げの手間を省いた軽い形)。

## 1. 今の RENS で、サブ MIP の中のダイブはどう回っているか (コードとログで確かめた)

### 1.1 コード

- 外側の根の RENS は 3 つの道に分かれる ([`Solver::rens`]、src/mip/heuristics.rs)。
  - 暫定解があるとき (既定): `ENOMOTO_MIP_SUBFIX` の `r` (既定 `rtcqf`) で [`Solver::subfix_dive`] (src/mip/heur_subfix.rs)。
    HiGHS の RENS と同じく、lock の順に LP 解の切り下げ・切り上げの範囲に締め、LP を解き直しながら目標の固定率 (初め 0.6) まで進めてから
    サブ MIP を解く。根で実行不能なら目標を半分にしてやり直す。`f` で、ダイブが解を出さなければ既定の RENS (`rens_old`、LP で整数値の列を全部固定) も 1 回。
  - 暫定解がない (か FJ の悪い解だけの) 根: 暫定解なしの RENS ([`Solver::rens_dive_with`]、目標の固定率 0.8、サブ MIP に残りの 4 割・最大 20 秒、やり直しなし)。
  - サブ MIP の中 (深さ 1 以上): 固定率 0.6 から、ノード上限 50・残りの 15%・最大 3 秒で最大 4 回。
- サブ MIP の根でも根のヒューリスティクスがそのまま回る (src/mip/solver.rs の根のヒューリスティクス)。ただし:
  - Feasibility Pump と 3 つのダイブ (分数・ベクトル長・係数) は **暫定解がない間だけ** (`noinc_like()`)。誘導ダイブは暫定解が要るので回らない。
    サブ MIP は外側の暫定解を打ち切り値としてしか持たないので、始めは暫定解なし。しかし、先に回る丸め (simple・ZI・shifting・randomized) や
    サブ MIP の中の入れ子の RENS が解を出すと、その後のダイブは回らない。
  - ダイブ 1 本の LP 反復の予算は外側と同じ形 (`ENOMOTO_MIP_RHBUDGET`: 0.05 × そのサブ MIP でここまでの LP 反復 + 1000)。
    サブ MIP の LP は小さいので、実際には 1000 前後。

### 1.2 ログ (`ENOMOTO_MIP_SUBMIP_VERBOSE=1`、4 スレッド・種 1・60 秒、`logs/*_smv_s1.log.gz`、ビルド ca7d8c9a1)

RENS が最終解の出どころになる問題 (analysis/rens_depth3_20261101.md) で数えた。

| 問題 | 外側の RENS | サブ MIP の中のダイブ (回った数 / 解を出した数) | RENS の解の出どころ (サブ MIP の中) |
|---|---|---|---|
| drayage-100-23 | 根の木で最適解 (0.1 秒) が先に出て、RENS まで来ない | 0 / 0 | - |
| dcmulti | SUBFIX RENS (固定率 0.62、自由な整数列 27/72)、NodeLimit 21 ノード、受け入れ | 0 / 0 (丸めが先に解を出し、ダイブは回らない) | 入れ子の RENS → randomized rounding・木の randomized rounding |
| mas76 | 根の被約費用・LP face・RENS のサブ MIP が並ぶ | 15 / 0 | int shifting・randomized rounding・木のダイブ (tree diving) |
| physiciansched6-2 | SUBFIX RENS (固定率 0.64、自由な整数列 1569/4417)、0 ノードで最適 49324 | 0 / 0 (根の LP が整数) | サブ MIP の根の LP |
| blp-ar98 | SUBFIX RENS (固定率 0.60、自由な整数列 6306/15794)、TimeLimit 89 ノード、受け入れ | 11 / 1 | 入れ子 (深さ 2) の RENS の中のベクトル長ダイブ、randomized rounding・木の randomized rounding |

- サブ MIP の中のダイブが RENS の解に効いたのは blp-ar98 の 1 回だけ (深さ 2 のベクトル長ダイブ)。
  多くは丸め (randomized rounding・int shifting) と、サブ MIP の木のダイブ・丸めから出ている。
- 丸めが先に解を出すとダイブが回らない (dcmulti)。外側の暫定解を打ち切り値に使うので、悪い丸めの解でもダイブを止めてしまう。

### 1.3 RENS が呼ばれない・打ち切られる場面 (LPBOX の少数問題の def の 42 回のログ、analysis/lpbox_20261122/logs/*_def_*)

- 暫定解なしの RENS (固定率 0.8): 14 回呼ばれて **14 回とも解なし** (30n20b8・mzzv42z・neos-1456979 の全回、roll3000 種 2・3、timtab1 の 1 スレッド)。
- SUBFIX RENS (暫定解あり): サブ MIP 28 回 (1 回目 23・やり直し 5)。

| サブ MIP の状態 | 解を受け入れた | 受け入れなし |
|---|---|---|
| Infeasible (根で実行不能、固定が強すぎ) | 0 | 7 |
| NodeLimit (ノード上限 500) | 6 | 10 |
| TimeLimit | 1 | 2 |
| Optimal | 2 | 0 |

- 呼ばれない場面: 「再スタートの前に解なし」で飛ばす 16 回 (neos-860300・roll3000・timtab1 の再スタートの後)、早い再スタート 1 回。
  hypothyroid-k1・nw04 は根のカットのループの中か木で解けて RENS まで来ない。
- 「RENS (more nodes)」(ノード上限で止まったサブ MIP の続き) は、並列のサブ MIP が上限の前に止まっていて 5 回飛ばされた。

### 1.4 案との違い

- 同じ所: サブ MIP の根では、RENS の近傍で分数・ベクトル長・係数のダイブが回っている (暫定解がまだないとき)。
- 違う所:
  1. 手間: サブ MIP の前処理・根の LP・カットのループを省ける。
  2. 回る条件: サブ MIP の中のダイブは、丸めが先に解を出すと回らない。誘導ダイブ (外側の暫定解へ寄せる) はサブ MIP では回らない。
  3. 近傍: 既定の SUBFIX は固定率 0.6 まで (lock の順に締めては LP を解き直す) で、案の「LP で整数値の列を全部固定」(`rens_old` と同じ) より緩い。
  4. 予算: サブ MIP のダイブは、サブ MIP の小さい LP の反復数に比例した予算になる。
- LPBOX の `d` (analysis/lpbox_20261122 の 4 節) は、箱の幅 k = 1 にすると案とほぼ同じ定義域になる
  (一般整数列 [⌊x̄⌋, ⌈x̄⌉]・LP で整数値の 0-1 列を固定・伝播)。違いは、一般整数列で LP 値が整数の列を固定しないことと、後に FJ・Local-MIP を回すこと。
  LPBOX の `d` では、ダイブが効いたのは blp-ar98 (分数ダイブ) と roll3000 (係数・誘導ダイブ) で、丸めは一度も解を出さなかった。
