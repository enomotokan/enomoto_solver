# canddet を既定にする (2026-11-02)

- canddet = 共有の待ち行列のノードの基底を正則に直し DSE 重みも渡す (pooldse、analysis/singular_fix_b_20261102.md・pool_dse_20261102.md)
  + FJ のスレッドの FJ_P3 の前処理にも止めの合図を渡す (FJSTOP、analysis/small_overhead2_20261102.md)
  + 裏のカットのスレッドのカットの LP 1 回の上限を時間ではなく反復で決める (`LP_WORK=1`、analysis/bg_lp_work_20261102.md)
  + 裏のカットを根のヒューリスティクスの長さではなく自分の停滞・手間の上限で止め、主は停滞まで待ち、上限に当たったら主が続ける (`DETERM=c`、analysis/bg_determ_20261102.md)。
- 根拠: `halve_submip_20261030.md` の bench18 (58 問 2 回 SGM −0.4%・解けた数 85 → 86、Mittelmann −0.6%・45 → 46、60 秒超えなし)。
  全体は揺れの範囲で、害のない作りの改善と、遅い機械・混んだ機械への備え (裏のカットの結果が機械の速さ・ヒューリスティクスの長さに左右されない) として入れる。
  問題ごとには、3 回とも同じ向きで neos-911970 (12〜40 → 8〜10 秒)・neos-3083819-nubu (17〜34 → 9〜15 秒)・air03・neos8 が速く、
  **mc11 (13〜27 → 23〜40 秒) と hypothyroid-k1 (32〜33 → 42〜45 秒) が遅い** (停滞まで待つ費用。速い機械では既定でも裏のカットが十分回っていた)。
  pooldse・FJSTOP だけの cand は +1.4% (bench18)、pooldse は bench17 で −1.0%、poolfix は bench16 で +1.8% (どれも揺れの範囲)。

## 既定にした内容と戻し方

| 設定 | 新しい既定 | 元の振る舞いに戻す |
|---|---|---|
| `ENOMOTO_MIP_POOL_BASIS_FIX` (`src/mip/solver.rs`) | 使う (`env_def!` `1`) | `=0` |
| `ENOMOTO_MIP_POOL_DSE` (`src/mip/solver.rs`) | `1` (`n` で重みのない分を厳密に埋める) | `=0` |
| `ENOMOTO_MIP_SMALL2_FJSTOP` (`src/mip/solver.rs`) | 使う | `=0` |
| `ENOMOTO_MIP_BG_CUTS_LP_WORK` (`src/mip/separation.rs`) | `1` | `=0` |
| `ENOMOTO_MIP_BG_CUTS_DETERM` (`src/mip/solver.rs`) | `c` | `=0` |

- 5 つを戻す並びは `analysis/halve_submip_20261030/run.py` の `"precanddet"`。これからの記録の `"def"` は canddet を含む新しい既定。
- 残っている課題: mc11・hypothyroid-k1 の待ちの費用 (停滞の窓・閾値を詰める、暫定解との差が十分小さければ待たない、など。未試作)。

## 確かめ (`canddet_default_20261102/check.py`、新しいビルドで 1 回ずつ。出力 `check.out`、ログ `logs/`)

| 問題 | 環境変数なし | 5 つを明示 | 元に戻す形 (`=0` ×5) |
|---|---|---|---|
| neos-911970 | 最適 13.7 秒 (裏のカット determ、待ち 0.82 秒) | 最適 10.0 秒 (determ、待ち 0.92 秒) | 最適 21.5 秒 (今までの待ち 0.28 秒、26 ラウンド) |
| air03 | 最適 4.4 秒 | 最適 3.9 秒 | 最適 4.7 秒 |
| mc11 | 最適 24.5 秒 (determ、待ち 2.97 秒) | 最適 26.9 秒 (determ、待ち 2.89 秒) | 最適 17.6 秒 (今までの待ち 3.89 秒、48 ラウンド) |

- 環境変数なしで `MIP: background cuts: determ BgDeterm {...}` が出て、5 つを明示した形と同じ振る舞い。既定が入った。
- mc11 は bench18 と同じく、元に戻す形より遅い (待ちの費用)。
- `cargo build --release` 通過。
