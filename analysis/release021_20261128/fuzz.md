# 0.2.1 に向けた誤答の確かめ (2026-11-28)

0.2.1 = 0.2.0 (main 7182f383a) + 誤答の直し 1 つだけ (9e11b2c4c の取り込み)。0.2.0 の後に既定にした改良 (PRR_TIGHT=o・DUALEQ・RSTLOOSE・PAR4ROLE=d など) は入れない。

- 直した誤り: プロービングの後の「幅の小さい列の固定」(`fix_tiny_width_columns`、src/mip/mod.rs) が、等式の行の破れの予算を元の右辺の大きさで測っていた。
  固定した列を右辺へ移した後の右辺が小さいと、後の伝播の許容を越え、実行可能な問題を「前処理で実行不能」と返すことがあった。
  直しでは、移した後の右辺でも予算を測り、小さい方を使う。
- 0.2.0 で再現した例: analysis/pregd_20261128/fuzz_domcol.py の種 4631 (8 列・6 行。HiGHS の最適 0.6868 を 0.2.0 は「実行不能」と返す)。
  0.2.0 でも `ENOMOTO_MIP_NO_TINY_WIDTH_FIX=1` か `ENOMOTO_MIP_DOMCOL=0` で避けられる。詳しくは docs/improvement_history.md の 2026-11-28 の節 (作業ブランチ) と analysis/pregd_20261128/README.md。

`bash analysis/release021_20261128/fuzz.sh` (この木の python/ を PYTHONPATH で使う) と cargo test の結果 (run.out):

| 確かめ | 件数 | 結果 |
|---|---|---|
| check_default (0.2.0 の基準) | 6 | 全部一致 (道筋は 0.2.0 と同じ) |
| fuzz_tiny 400 0 / 400 1000 | 800 | bad 0 |
| mip_fuzz_vs_highs 300 | 300 | bad 0 |
| fuzz_objint 1000 0 / 1000 5000 | 2000 | bad 0 |
| fuzz_slack 1000 0 / 1000 5000 | 2000 | mismatches 0 |
| fuzz_domcol 10000 0 | 10000 | 種 4631 は正しく解ける。食い違いは種 2880・6174・6532 の 3 件で、どれも当方の誤りではない (2880・6174 は HiGHS の前処理の誤りで、前処理を切った HiGHS は当方と一致。6532 は許容誤差の差 4e-6)。作業ブランチの確かめ (analysis/pregd_20261128) と同じ |
| cargo test --release --lib | — | 426 通過・0 失敗・19 無視 |
