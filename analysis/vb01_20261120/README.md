# decomp2 型の解探し: 0-1 列どうしの含意を使う vbounds (SCIP-VB、2026-11-20)

親の依頼 (analysis/slowm_20261119/README.md 5 節): decomp2 では SCIP が前処理の直後の vbounds ヒューリスティクスで最適解 −160 を出す
(2.2 秒) が、当方の vbounds (`heur_scip.rs` の `vbounds_heur`、`VarBounds::from_rows`) は連続列と整数列の 2 列の行しか使わず、
0-1 列だけの問題では何もしない (14.1 秒・648 ノード)。0-1 列どうしの含意・クリークを変数上下限のグラフに入れる形を試作する。

## 1. SCIP の heur_vbounds (事実、SCIP の heur_vbounds.c を読んだ)

- 境界の節点 (列ごとに下限・上限の 2 つ) のグラフを、変数上下限 (VLB・VUB) と **クリーク表** (0-1 列の含意。SCIP では 0-1 列どうしの
  `x − y ≤ 0` もクリーク {x, ¬y} として入る) から作る。逆向きのグラフ (節点 → それを動かす側) を節点の番号の順に深さ優先でたどり、
  帰りがけ順 (辺のない節点だけの木は入れない) を位相順とする。影響を与える側が先に来る。
- 型: 緩める型 (feasvariant、既定 6 = 目的の良い側だけ・悪い側だけ) と締める型 (tightenvariant、既定 7 = 目的を見ない・良い側・悪い側) の
  最大 5 通りを、この順に回す。締める型は節点の境界を動かす側 (伝播が起きる) に、緩める型は動かさない側に固定する。目的の良い側は
  費用 ≥ 0 なら下限。目的を見ない型の固定がすべて良い側 (悪い側) なら、その型は同じなので回さない (skipobj)。
- 固定ごとに伝播 (2 巡まで)。矛盾したら直前の列を反対の値にし、それも矛盾すればその型はやめる。反対にした回数が 10 を超えたら固定をやめる。
- 節点の数が列の数 × 0.65 未満なら使わない。固定の後、整数列の固定率 0.65 以上 (minintfixingrate)、LP を解いて丸めて実行可能ならそれで終わり、
  そうでなければサブ SCIP (5000 ノード) を、その前処理の後に全列 (連続列も) の固定率 0.65 以上 (minmipfixingrate) なら解く。
- 呼ぶ時: 前処理の後・根の最初の LP の前 (HEUR_TIMING_BEFORENODE、freq 0)。
- decomp2 (SCIP 10.0.2、1 スレッド): 2.39 秒・1 ノード。vbounds が LP 8 反復の時点で −160 を出す (ヒューリスティクスの統計で vbounds だけが解を出す)。

## 2. 試作 `ENOMOTO_MIP_VB01` (既定では使わない)

- `heur_scip.rs` の `vbounds01_heur` と `vb01_fix`。1 節のとおりになぞる。グラフは衝突グラフ ([`clique_graph_get`]、行から作る組と
  前処理の含意 `conflict_edges`)。文字 `2j` (x_j = 1) を下限の節点、`2j+1` を上限の節点とし、節点 a の逆向きの後続は ¬a の隣
  (a を導く文字)。伝播は `conflict_edges` の含意つきの写しの定義域で行う。固定率は整数列 0.65 以上かつ全列 (定義域で固定された列で数える) 0.65 以上で、
  `solve_submip` (500 ノード、LP から) を解く。
- 置き場所 (`solver.rs` の `vb01(c)`):
  - `b`: 前処理の後・根の LP の直前。SCIP の BEFORENODE に当たる。再スタートの後の根では回さない (1 回の求解で 1 度)。
    SCIP-GATE (analysis/scipgate_20261120) の `ENOMOTO_MIP_VBCLQ` (pseudoobj) はこのすぐ後に置き、VB01 の解を上限に使う。
  - `r` (値 `1` も同じ): 根の SCIP 由来のヒューリスティクス (locks・clique・vbounds) の後。
- Solver を使わない版 `vbounds01_on_problem(p, verbose, time_limit) -> Option<(目的値, 解)>` (SCIP-GATE の依頼。前処理の後・前処理の
  LP probing の前の `ENOMOTO_MIP_VBCLQ=e` から呼ぶ)。並べ替え (`vb01_order`) と固定・伝播 (`vb01_fix_dom`) は Solver 版と共有し、
  サブ MIP だけ `solve_problem` (500 ノード、それまでの最良の値を打ち切り値に) で解く。単体テスト `vb01_on_problem_finds_feasible`。
- 手間: グラフの辺の数に比例する並べ替えと、型ごとの伝播 (`fix_then_submip` と同じ上限: 非零数 × 20 + 100 万) とサブ MIP (500 ノード)。
- 全列の固定率の条件は、M の数え上げの 1 回目 (`count_M_b_v1.jsonl`) で neos-5188808-nattai が整数列だけ 0.8 固定してサブ MIP を
  5 本とも時間切れにし、VB01 に 12.4 秒かかったので足した (SCIP の minmipfixingrate)。

## 3. 対象の問題での効き (1 スレッド、`logs/`)

| 問題 | 既定 | VB01 `r` | VB01 `b` | SCIP |
|---|---|---|---|---|
| decomp2 | 23.6 秒・887 ノード | 3.7 秒・0 ノード | 1.65 秒・0 ノード | 2.4 秒・1 ノード |
| hypothyroid-k1 | 10.5 秒・0 ノード | 11.0 秒 | 11.2 秒 (根の LP の前に −2692) | 8.2 秒 |

- decomp2: 締める型 (目的を見ない) のサブ MIP が −160 (= 根の下界) を出し、根で閉じる。4 スレッドでは既定でも Local-MIP が 0.9 秒で −160 を
  出すので差は小さい (既定 4.0 秒、`r` 2.4 秒)。
- hypothyroid-k1: 根の LP の前に −2692 (SCIP の vbounds は −2633) を出すが、全体の時間は前処理の LP probing が占めるので変わらない。
  SCIP-GATE の pseudoobj + LP probing を省く形 (`vbcl`) と組んで 3.3〜3.6 秒 (SCIP-GATE の記録)。
- nw04 (1 スレッド、60 秒): 既定 17.3 秒、`b` 29.2 秒。VB01 は 4.8 秒 (87482 列のサブ MIP)。残りの差は、暫定解が変わってカットの LP が
  2 巡目から分かれ、既定では 4 巡目に LP が整数解に当たっていたのが当たらなくなったため (道筋の分かれ)。

## 4. 働く問題の数え上げ

- 構造 (`count_struct.py`、元の問題): 衝突グラフに辺を持つ 0-1 列が整数列の 0.65/2 倍以上 = P 29/58 問、M 113/186 問。
- 実際に回した数え上げ (`count.py`、1 スレッド・種 1・60 秒、ログの VB01 の行):

| 組 | 回った | サブ MIP まで行った | 解を出した | VB01 の時間 (最大) |
|---|---|---|---|---|
| P `r` (`count_P.jsonl`、全列の条件の前) | 25 | 20 | 2 (misc07・cvs16r128-89) | 0.13 秒 |
| P `b` (`count_P_b.jsonl`) | 58 | 23 | 12 | 4.63 秒 (nw04、解を出した) |
| M `b` (`count_M_b.jsonl`、構造の候補 113 問) | 112 | 61 | 32 | 4.23 秒 (sorrell3、解を出した) |

- `r` は根のヒューリスティクスまで行き、しかも暫定解と根の LP の差が 1% 以上のときしか回らない (多くの問題はその前に解ける・時間切れ)。
- M `b` で解を出さずに時間を使った問題: nursesched-sprint02 3.4 秒、cryptanalysiskb128n5obj14/16 1.8 秒、supportcase40 1.8 秒 (600 秒の上限に対して小さい)。
  113 問の VB01 の時間の合計は 40.8 秒。
- 「解を出した」は根の LP の前の暫定解になったという意味で、全体が速くなるかは計測で見る。

## 5. 誤答の確かめ

- `fuzz.sh` (release019 の fuzz 7 本 + cargo test を `ENOMOTO_MIP_VB01` つきで): `=1` で 7 本とも bad 0、cargo test 412 通過。
  `=b` は 7 節。
- `fuzz_vb01.py`: 0-1 列だけで含意・AND 型・集合分割/詰め込み・ナップサック・被覆の行を持つ問題を作り HiGHS と比べる。小さい問題は前処理か
  根で閉じて VB01 まで届かないので、列 40〜250・行 2n〜4n にしている (40 問で VB01 が回ったのは 14 問)。

## 6. 計測の依頼 (台本と分担表)

- 設定 (`cfg.py`、どれも 4 スレッド): `def` (今の既定)、`vb` (`ENOMOTO_MIP_VB01=b`、SCIP-GATE の `vb` と同じ設定・同じ名前)、`vb01r` (`=r`)。
- P: `python run.py P K` (K = 0〜5、`shards_P.json`、58 問 × 種 1〜3 × {def, vb, vb01r}、60 秒、各 0.7 時間)。
- M: `python run.py M K` (K = 0〜7、`shards_M.json`、`count_M_b.jsonl` でサブ MIP まで行った 61 問 × 種 1・2 × {def, vb}、600 秒、各 3.5 時間)。
- 照合: `python ci.py 60 def vb res_P_s*.jsonl` など。
- SCIP-GATE の計測も `def`・`vb` を含むなら重なるので、どちらか一方で回して混ぜて読む。

## 7. 誤答の確かめ (全列の固定率の条件と「再スタートの後は回さない」を足した後、`VB01=b`)

- `VB01=b bash fuzz.sh`: fuzz 7 本とも bad 0 (tiny 400・highs 300・objint 1000・slack 1000・tiny2 400・objint2 1000・slack2 1000)、
  cargo test 413 通過。`fuzz_vb01.py 500 0`: bad 0 (VB01 が回ったのは 455 問)。
- Solver を使わない版に切り出した後も、decomp2 (1.46 秒・0 ノード) と hypothyroid-k1 (根の LP の前に −2692) は同じ。
