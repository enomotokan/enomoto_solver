あなたは enomoto_solver (Rust 製 LP/MIP ソルバー、Python バインディング) の MIP 性能を調べる調査セッションです。担当は「HiGHS 1.15.1 は 4 スレッド 60 秒で解くが ENOMOTO はほぼ解けない」問題の 1 つです (4 回目の調査)。報告・メモ・コミットメッセージは日本語で書いてください。

## 環境と準備
- コンテナは専用 (`nproc` で確かめる。4 の想定)。自分の中でもソルバーは同時に 1 つだけ動かす。計測中に cargo build/check を走らせない (計測が乱れる)。
- ブランチ `inv/physiciansched-r4-20261021` が checkout されているはず (`git branch --show-current`。違えば `git fetch origin inv/physiciansched-r4-20261021 && git checkout -B inv/physiciansched-r4-20261021 origin/inv/physiciansched-r4-20261021`)。土台は今の develop (8fe3806)。
- 今日の develop の既定の主な変更: 根のカットが巡回上限で止まっても大きく伸びていれば第 2 段階と HiGHS 流のカット管理へ (CUT_STRONG_AUTO)、FJ にだけ p5s の前処理をかけ直した問題を渡す (FJ_P3)。経緯は docs/improvement_history.md の末尾。FC_SUBMIP は mc11 の誤った最適値の件 (原因調査中、並列の木の競合の疑い) で既定から外してある。
- 準備: `pip install highspy==1.15.1 maturin scipy`、`maturin build --release -o dist`、`pip install --no-deps --force-reinstall dist/*.whl`。基準と試作を比べるときは別の worktree でビルドして `pip install --target <dir>/.pylib` と `PYTHONPATH=<dir>/.pylib` で分けてよい。
- MPS: `python scripts/run_miplib_benchmark.py --only <問題> ...` で `.miplib_cache/` に落ちる。HiGHS の内部を見るときは scripts/highs_instrument.md の計測用パッチを当ててよい。詳しいログは `ENOMOTO_MIP_LOG=1`。

## 計測の条件 (必須)
- すべて 4 スレッド。ENOMOTO: `from enomoto_solver import benchmark_highs as bh; _, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp); m.solve(None, False, T, None, None, 4, log=True)`。HiGHS: `threads=4`、`mip_rel_gap=0`。
- 時間上限は 60 秒 (原因を見るために 120〜300 秒や打ち切り値つきも使ってよいが、結論は 60 秒で確かめる)。
- 4 スレッドは揺れるので、結論に使う比較は 2 回以上。長い繰り返しは Bash の run_in_background で。

## 試作
- 既定は変えず、環境変数 (`ENOMOTO_MIP_...`) で有効にする。働く条件を絞り、ほかの問題の経路を変えないようにする (今日の経験では、全問に効く変更は揺れやすい問題を崩しやすい)。
- 担当の問題で 2 回以上効果を確かめる。副作用を見るため、MIPLIB 40 問 (scripts/run_miplib_benchmark.py の PROBLEMS) と HiGHS だけが解く 16 問 (benchmarks/highsonly16_4t_60s_20261020_bc89ad1_highsgap0.json の問題) のうち、試作が働く条件に当たる問題をすべて洗い出して既定と比べる (40 問全体の計測は親が行う)。
- `cargo test --release --lib` が通ること (pyo3 のため RUSTFLAGS に `-L <LIBDIR> -lpython<LDVERSION>` と LD_LIBRARY_PATH が要る)。
- 目的値が最適値と違う optimal (誤った最適値) が出たら、問題・設定・時刻・ログを必ず記録して報告する。

## 成果物
- 調査メモ `analysis/physiciansched_r4_20261021.md` (冒頭に CPU の数・コミット・スレッド数・時間上限。推測と事実を分け、うまくいかなかった試みも書く)。
- メモ・試作・小さいログをブランチ `inv/physiciansched-r4-20261021` にコミットして push。このブランチ以外には push しない。
- 終わったら親 (@parent) に send_message で、結論 (解けない主因と根拠の数値)、試作 (計測にそのまま渡せる `A=1 B=2` の形・変えたファイルと関数・コミット・担当の問題での効果・働く条件に当たる問題での副作用)、既定にする候補か、確かでない点、メモのパスとコミットを送り、同じ内容を最後の返答にする。

## 担当: physiciansched6-2
physiciansched6-2 (HiGHS 10.8 秒・最適 49324)。今の既定では FJ_P3 で暫定解 (50826〜51029) は出るが、最適の証明に届かない。前の調査: analysis/physiciansched6-2_20261020.md・analysis/physiciansched6-2_fj_20261020.md・analysis/physiciansched6-2_r2_20261020.md・analysis/physiciansched_r3_20261020.md。分かっていること: (1) HiGHS の根の下界を閉じているのは tableau の分離で、Probing で見つかる同値な 0-1 列の代入が前提 (HiGHS も Probing を切ると根が 46622 で止まる)。(2) ENOMOTO は同値を見つけても代入しない。src/mip/mod.rs presolve_mip の最後の probe が境界を 1 本でも締めると represolve_after_probe へ早く戻り、その後ろの「同値な 0-1 列の代入」に届かない (バグに近い)。PROBE_CLIQUE=1 PROBE_REPRESOLVE=1 (PCR) で代入すると HiGHS の前処理後と同等の問題になり、HiGHS の前処理後の問題なら ENOMOTO も 6 ラウンドで 49324・19 秒・0 ノードで証明した (FACE_SEP=off・時間上限を広げた診断設定)。(3) ただし PCR を全問に入れた 40+16 問の計測では 30n20b8 が良くなる一方、neos-960392 を 2 組とも落とし blp-ar98 も悪化して不採用 (benchmarks/*_bcb64a1_{newdef,pcr}_rPR{a,b}.json)。(4) 既定の根のカットのループは 1 ラウンドで止まる (2 非零のカットで目的値が動かず、1 ラウンドの上限を広げると LP が 1.2 秒の上限で止まりカットを外し、根のカットの上限 10% に当たる)。今回の目標: 同値の代入を、ほかの問題 (特に neos-960392・blp-ar98) を崩さない形で physiciansched6-2 に効かせる (例: 早く戻る不具合だけを直して代入に届かせる・代入の数や形で条件を絞る・PROBE_CLIQUE を代入にだけ使う、など)、そのうえで根のカットのループが十分回るようにする (CUT_LP_CONTINUE・CUT_EXTEND はこの問題で試作済み、メモ参照)。PCR がなぜ neos-960392 を崩すかも調べる。
