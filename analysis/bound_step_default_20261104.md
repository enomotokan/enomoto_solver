# 報告の下界を目的値の刻みで切り上げる (BOUND_STEP) の既定化 (2026-11-04)

- 既定にした: `ENOMOTO_MIP_BOUND_STEP` (戻し方 `=0`、run.py の `prebs`)。目的値が刻みの格子にしか乗らない問題で、利用者に返す下界と経過の行の下界を
  格子の点に切り上げる (最大化は切り下げる)。探索・刈り込みは変えない (刈り込みは前から刻みを使う)。
- 根拠 (`analysis/bound_step_20261104.md`): supportcase18 の下界 47.187 → 48 (HiGHS の報告と同じ)。58 問で解けた数 44 → 44、下界が =opt= を越えた問題 0。
  `fuzz_tiny.py 400 0` は 13 件で同じ seed、`mip_fuzz_vs_highs.py 300` は 0 件のまま。最大化・目的の定数の問題でも最適値を越えない。
- 確かめ (`bound_step_default_20261104/check.out`、supportcase18・30 秒): 既定は下界 48.0、`ENOMOTO_MIP_BOUND_STEP=0` は 47.1867。
  `cargo test --release --lib` 339 通過・0 失敗。
- まだの確かめ: Mittelmann のような広い問題 (次の bench でまとめて見る)。費用のある連続列が残る問題 (bppc4-08) は切り上げない。
