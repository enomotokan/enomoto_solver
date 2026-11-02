# flbgx を既定にする (2026-11-02)

- flbgx = 実行可能性問題 (前処理後の目的関数が全部 0) で、前処理の LP による固定の前に FJ2 と Local-MIP を 3 対 1 のスレッドで並べる早めの探索
  (`ENOMOTO_MIP_FEAS_FJ_EARLY=fl`、`analysis/feas_early_lmip_20261102.md`) と、根の LP が 1 秒以上の問題で裏のカットの写しの根の LP に主の列の値も
  渡す形 (`ENOMOTO_MIP_BG_CUTS_BASIS_X=lpw`、`analysis/phys6_20261101.md` の 1 節)。
- 根拠: `halve_submip_20261030.md` の bench14 (58 問 2 回で SGM −4.0%・解けた数 82 → 88、Mittelmann −2.4%・47 → 49、片方だけ解けた回は
  flbgx 側だけ: ns1116954・physiciansched6-2 3/3・fhnw-binpack4-48 など)。

## 既定にした内容と戻し方

| 設定 | 新しい既定 | 元の振る舞いに戻す |
|---|---|---|
| `ENOMOTO_MIP_FEAS_FJ_EARLY` (`src/mip/mod.rs`) | `fl` (`env_def!`) | `=0` (`=1` なら FJ2 だけ) |
| `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS` | スレッドの 4 分の 1 の切り捨て、少なくとも 1 本 (4 本で FJ 3 + Local-MIP 1。c675564d) | 本数を数で指定 |
| `ENOMOTO_MIP_BG_CUTS_BASIS_X` (`src/mip/solver.rs`) | `lpw` (`env_def!`) | `=0` (`=1` なら根の LP の時間によらず渡す) |

- 2 つを戻す並びは `analysis/halve_submip_20261030/run.py` の `"preflbgx"`。これからの記録の `"def"` は flbgx を含む新しい既定。

## 確かめ (`flbgx_default_20261102/check.py`、新しいビルドで 1 回ずつ。出力 `check.out`、ログ `logs/`)

| 問題 | 環境変数なし | flbgx を明示 | 元に戻す形 |
|---|---|---|---|
| ns1116954 | 最適 6.0 秒 (早めの探索) | 最適 6.2 秒 (早めの探索) | 最適 55.9 秒 |
| fhnw-binpack4-48 | 最適 1.6 秒 (早めの探索) | 最適 1.5 秒 (早めの探索) | 時間切れ・解なし |
| physiciansched6-2 | 時間切れ (49424) | 時間切れ (49324) | 時間切れ (49324) |

- 環境変数なしで早めの探索・写しの列の値 (`background cuts: basis with the column values passed`) が働き、flbgx を明示した形と同じ振る舞い。
- physiciansched6-2 は今回 3 形とも時間切れ。主の根のカットのループが時間の上限で 4 ラウンド (400 本、48957) で止まる道 (bench14 で解けた回は
  別の道と見られる)。写しの根の LP は 1 反復 (`bgcopy 1 lps 1 iters`、元に戻す形は 1752 反復) になっているが、**列の値を渡した 2 形とも、写しが
  15〜32 秒の間に止めの合図の時点でラウンドの記録が 0 (`rounds so far []`、終わりに 2 ラウンド・下界は上がらず)**。元に戻す形は同じ間に
  2 ラウンド (48857 → 49007)。1 回だけの観察で、bench14 の 3/3 (と phys6_20261101.md の 5/5) と食い違うので、写しのカットのラウンドが
  列の値を渡すと進まない回があるのかを別に調べる。
