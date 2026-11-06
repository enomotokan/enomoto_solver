# 対称性の検出の直し (sym) と基底の受け渡しの直し (lp) を既定にする (2026-11-06)

既定にした (それぞれ `=0` で以前の形。まとめて戻すには run.py の `presymlp` =
`ENOMOTO_MIP_HO3_SYMNORM=0 ENOMOTO_MIP_HO6_SYMWORK=0 ENOMOTO_MIP_HO6_SYMREFINE=0 ENOMOTO_MIP_HO4_BGNB=0 ENOMOTO_MIP_TREE3_RBASIS=0`):

| 環境変数 | 場所 | 中身 | 出どころ |
|---|---|---|---|
| `ENOMOTO_MIP_HO3_SYMNORM` | `src/mip/mod.rs` | 対称性の検出に渡す問題だけ、前処理でまとめた幅つきの行の向き (-1 倍) をそろえる | `analysis/highs_only_20261105/G3.md` (fastxgemm-n2r6s0t2 が 600 秒で 2 回とも最適、HiGHS より速い) |
| `ENOMOTO_MIP_HO6_SYMWORK` | `src/mip/symmetry.rs` | 対称性の検出の時間・手間の上限を、色の細分化の後の探索の分だけに数える | `analysis/highs_only_20261105/G6.md` (uccase12・unitcal_7 で上限を細分化だけで使い切り、探索に入っていなかった) |
| `ENOMOTO_MIP_HO6_SYMREFINE` | `src/mip/symmetry.rs` | 探索の中の細分化を、色が変わった列から増分で回す | 同上 |
| `ENOMOTO_MIP_HO4_BGNB` | `src/mip/solver.rs` | 裏のカットの LP を主に入れ替えるとき、基底に加えて非基底の列の置き場所 (上限・下限) も渡す | `analysis/highs_only_20261105/G4.md` (satellites2-60-fs で主の解き直し 385 秒 → 1 反復) |
| `ENOMOTO_MIP_TREE3_RBASIS` | `src/mip/solver.rs` | 再スタートの後の根の LP で、前の基底と合う行が 0.9 x 行の数未満ならその基底を捨てて全スラック基底から解く | `analysis/tree_close3_20261104.md` (mzzv11 で再スタートの後の根の LP が約 1/3) |

## 根拠 (bench28、`analysis/halve_submip_20261030.md` の bench28 の節)

4 スレッド・60 秒、58 問 2 回 + Mittelmann 186 問。

| 設定 | 58 問 全回 (解けた数) | Mittelmann 186 (解けた数) | Mitt 片方以上解 |
|---|---|---|---|
| 既定 (当時) | 15.42 (88) | 42.09 (50) | |
| sym | −0.7% (88) | +0.0% (51) | +0.1% |
| lp | +1.2% (88) | −0.8% (52) | −4.1% |

- sym は neos-957323 が sym だけ解ける。lp は neos-1171737・timtab1 が lp だけ解ける。どちらも既定だけ解けた回はない。最適値の食い違い・異常終了は 0。
- sym と lp を合わせた形だけの計測はしていない (all28 は prr を含み 58 問 +4.8% だったが、その分は prr とみる。prr は別に調べる: `analysis/prr_harm_20261106.md`)。

## 確かめ (この既定化のビルド)

- 10teams・mzzv11 を 4 スレッド・60 秒で既定と `presymlp`: 既定のログには SYMWORK の行 (細分化の時間・手間) が出て、mzzv11 では RBASIS の行も出る。`presymlp` では出ない。
  (10teams は既定 3.5 秒・177 ノード、presymlp 4.7 秒・0 ノードでどちらも最適 924。mzzv11 はどちらも 60 秒で時間切れ -21548 / -22052。)
- `cargo test --release --lib`: 342 通過、失敗 0、無視 18。
