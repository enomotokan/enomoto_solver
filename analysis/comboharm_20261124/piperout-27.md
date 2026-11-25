# piperout-27 (combo の計測で 4 スレッドだけの害: 12.2 → 29.3 秒、0 ノード)

切り分け: 分担 0 の子 (VB)、2026-11-25。ビルドは 12a360ac8 の `pip install -e .`。
- このビルドには直し k・s・POLISH_STRICT が入っている。HEUR_ADAPT の 1m (ff7c0f745) と LMIP_SCORE_TICKS は入っていない。
- check_default は 6 問一致。

回したもの (結果は `res_t4only.jsonl`・`res_t4only_ks.jsonl`):
- 種 1・600 秒。
- 4 スレッドは 1 本ずつ回した。1 スレッドは 2 本ずつ並べて回した。

どの回も根で解ける (0 ノード、8124)。差は根の手間だけ。

| 形 | 1 スレッド | 4 スレッド |
|---|---|---|
| 既定 | 22.6 秒 | 15.7 秒 |
| combo | 27.3 秒 | 32.0 秒 |
| combo − ppi | | 33.5 秒 |
| 既定 + ppi | | 15.9 秒 |
| combo − sbpb・cutu・bfrt・nra・tie | | 32.5〜33.9 秒 |
| combo − heur | | 22.0 秒 |
| **combo − enum** | | **16.6 秒** |
| 既定 + enum | | 22.4 秒 |
| 既定 + heur | | 16.1 秒 |
| **既定 + enum + heur** | | **33.4 秒** |
| combo、直し k・s (`NODEROWS_ADAPT=tk ENUM_ADAPT=1s`) | | 25.3 秒 |
| 既定 + enum、直し s | | 24.3 秒 |

結論:
- 原因は **enum と heur の重なり**。ppi (4 スレッドだけで効く部品) ではない。
  - 既定に enum だけ足すと +6.7 秒、heur だけ足しても変わらない。両方足すと combo と同じ 33 秒になる。
  - combo から enum を抜くと既定並み、heur を抜くと半分戻る。
- 1 スレッドの差 (22.6 → 27.3 秒) は、並べて回した揺れの中だが、向きは同じ。
- 直し s は害を一部だけ消す (combo 32.0 → 25.3 秒)。既定 15.7 秒までは戻らない。
- 仕組み (ログ) は見ていない。heur の分が、先端の HEUR_ADAPT の直し 1m (FP の第 3 段) または LMIP_SCORE_TICKS (app1-1) で消えるかは、ビルドを作り直して確かめる (下の追記)。
