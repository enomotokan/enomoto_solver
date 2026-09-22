# `commit_update` のアロケーション除去 (A-1 / A-2)

対象: `FtLu::commit_update` (`src/simplex/lu.rs`) が Forrest-Tomlin 更新 1 回ごとに
払っていた小さな無駄 —— ボトルネック分析で挙がった **A-1**(毎回の `Vec` 確保)と
**A-2**(`HybridVec::indices()` の `Box<dyn Iterator>`)。

`try_update` 側は既に `scratch_a_tilde` / `scratch_e_tilde` の再利用で自前の
アロケーションを消していたが、両呼び出し元が合流する `commit_update` には
1 更新あたり最大 3 本のヒープ確保が残っていた。`ENOMOTO_PROF_PHASES_EXT` が
`ft_update` を wall time の 13〜20%(問題による)と報告していたうちの残り分。

## 変更内容

### A-1: 中間 `Vec<(usize, f64)>` を作らない (`HybridVec::pack_scaled_dense`)

変更前の `commit_update` は、更新ごとに

```rust
let r_vec: Vec<(usize, f64)> =
    (0..m).filter(|&i| i != p).map(|i| (i, -old_pivot * e_tilde[i])).filter(|&(_, v)| v != 0.0).collect();
// ...
let off_diag: Vec<(usize, f64)> =
    (0..m).filter(|&i| i != p && a_tilde[i] != 0.0).map(|i| (i, a_tilde[i])).collect();
```

と 2 本の `Vec` を `collect()` で作り、そのまま `HybridVec::pack` に渡していた。
`pack` は sparse 側ではその `Vec` をそのまま保持するが、**dense 側では
`vec![0.0; len]` をもう 1 本確保してそこに散布し、渡された `Vec` は捨てる**。
つまり dense に倒れた更新では確保 2 回 + 破棄 1 回、sparse でも
`collect()` の容量再確保(フィルタ付きイテレータは上限しか申告できないため
幾何級数的に伸ばす)を毎回払っていた。

この 2 本はどちらも「密な配列 `src` を定数倍し、自分のピボットスロット
`skip` を除き、0 になった成分を落とす」という同じ形をしている
(`r_vec` は `src = e_tilde, scale = -old_pivot`、`off_diag` は
`src = a_tilde, scale = 1.0`)。そこで `HybridVec` に

```rust
pub fn pack_scaled_dense(src: &[f64], skip: usize, scale: f64, dense_fraction: f64) -> Self
```

を追加し、ペア列を一度も実体化せずに直接 `HybridVec` を組み立てるようにした。

- 先に nnz を数える 1 パス(`skip` はループ内で分岐せず、後から 1 引く)。
  sparse/dense の選択に nnz が先に要るため。連続した `f64` 配列の直線走査で、
  置き換える述語付き `collect()` より安い。
- dense 側: `src.to_vec()`(memcpy)→ 必要なら定数倍 → `data[skip] = 0.0`。
  `Vec<(usize, f64)>` を一切確保しない。散布ではなくコピーなのでアクセスも連続。
- sparse 側: `Vec::with_capacity(nnz)` の 1 回確保のみ。再確保なし。

さらに `dot`(新ピボット判定に使う `r_vec · a_tilde`)は、作った `R` eta に対する
`dot_dense` そのものなので、eta を先に作ってそこから読むようにした。
変更前も判定前に `r_vec` を丸ごと作っていたので、棄却される更新のコストは増えない。

### A-2: `indices()` の `Box<dyn Iterator>` を廃止

`removed.off_diag.indices()` は `commit_update` からの 1 箇所でしか使われて
いないが、`Box<dyn Iterator<Item = usize>>` を返すため更新ごとにヒープ確保 +
index ごとの動的ディスパッチを払っていた。ジェネリックなクロージャを取る
`for_each_index(&self, f: impl FnMut(usize))` に置き換え、両 arm が
単相化されてインライン展開されるようにした(dense arm が配列全体を走るのは従来通り)。

## 等価性

数値的に変更前と**完全に同じ**であることを意図した変更で、実際にそうなっている。

- `pack_scaled_dense` が「元の `collect()` → `pack`」と一致することを
  `sparse.rs` のユニットテスト `pack_scaled_dense_matches_collect_then_pack` で
  直接確認(両 arm、スケール後に 0 に潰れる成分、skip スロット、
  dense arm が skip 位置に 0.0 を保つ規約を含む)。`cargo test --release --lib` 223 件すべて green。
- NETLIB 93 問題で **目的関数値が 93/93 ビット単位で一致**、ステータスも全問 `optimal` で不変。
- `ENOMOTO_PROF_PHASES_EXT` の反復回数・refactor 回数も完全一致
  (d2q06c 6383 iters / 43 refactor、pilot87 6735 / 62、greenbeb 5445 / 27 — 前後で同値)。
  CLOCK トリガの決定性(tick 列が同一なら同じ反復で refactor する)が保たれている裏付け。

## 計測

### `ft_update` フェーズ(`ENOMOTO_PROF_PHASES_EXT`、1 回計測)

| 問題 | before us/iter | after us/iter | 変化 |
|---|---|---|---|
| d2q06c  | 23.146 | 22.220 | **-4.0%** |
| pilot87 | 37.335 | 34.628 | **-7.3%** |
| greenbeb| 16.375 | 15.285 | **-6.7%** |

### NETLIB 93 問題フル(求解時間、各問題 3 回実行の最小値)

同一コンテナ・同一 netlib キャッシュで、変更の前後をそれぞれ 93 問題走らせた。
HiGHS は `.mps` の読み込みにのみ使い、計測対象は本ソルバの `model.solve()` のみ。

- 合計: **40.1174s → 40.0075s (0.9973x, -0.1099s)**
- **10% 以上退行した問題: 0 件**(採用基準を満たす)
- 最悪の比: `ship12s` 1.040x(0.0278s → 0.0289s、+1.1ms — 測定ばらつきの範囲)。
  絶対値で最大の増加は `dfl001` の +0.214s (1.012x)。
- 改善が大きい問題: `afiro` 0.613x、`scagr7` 0.719x、`sc205` 0.869x(いずれも
  ミリ秒未満〜数ミリ秒の問題で、比としては大きいが絶対値は小さい)。
  絶対値では `pilot87` -0.134s、`pilot` -0.064s、`maros-r7` -0.043s。

FT 更新は 1 反復あたり数十マイクロ秒のフェーズなので、全体への寄与は
0.3% 程度に留まる。ただし退行ゼロ・数値挙動完全一致で得られる分であり、
`ft_update` フェーズ自体では 4〜7% 削れている。

## 対処しなかった項目と理由

分析で挙がった **B-1**(`u_transpose_sweep` の needed 判定で eta を全走査)と
**B-2**(`l_solve_into` の dense FTRAN 先頭の `O(m)` permute gather)は、
分析自身の結論どおり今回は手を付けていない。

- B-1: 固定の `O(num_row)` 項は HiGHS の `btranU` も同じ形で持っており、
  reach-set の順序付きリストを持たない現設計では除去が難しい
  (過去に GP 疎求解を試して負けている)。
- B-2: dense FTRAN パスは密度ゲートで意図的に dense を選んだときにだけ走るので、
  先頭の `O(m)` gather は本質的コスト。疎側の `solve_sparse_into` では既に回避済み。
