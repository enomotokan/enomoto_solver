# グラフの自己同型 (対称性の検出) のソフトウェアの調査

調査日: 2026-11-10。目的: ENOMOTO (Rust の MIP ソルバー、MIT ライセンス、pyo3 の cdylib として wheel で配る) の対称性の検出
(行と列の二部グラフ + 色の細分化 + 後戻りなしの貪欲な個別化) を外部の検出ライブラリに置き換える・補う価値があるかの材料。

## 確かめ方と届かなかったもの

- 届いた: `raw.githubusercontent.com`、`api.github.com/search/*`、`crates.io` の API、`static.crates.io` (クレートの中身)、`arxiv.org/pdf`。
- 届かなかった (プロキシで 403): `pallini.di.uniroma1.it` (nauty・Traces の公式)、`users.aalto.fi` (bliss の公式)、
  `users.cs.northwestern.edu`・`vlsicad.eecs.umich.edu` (saucy)、`codeload.github.com` (tar 一式)。
  `api.github.com/repos/<owner>/<repo>` もこのセッションでは許されていない。
  → nauty はクレート `nauty-Traces-sys` に同梱の原本 (2.9.3) と SCIP 同梱の COPYRIGHT、bliss は `scipopt/bliss` (公式の fork) と
  `ds4dm/Bliss`、saucy は `andandandand/saucy-repo` (saucy 2.1) と `hrbrmstr/saucy` (R の包み) で確かめた。
- 読んだ論文 (pdftotext): McKay・Piperno "Practical graph isomorphism, II" (arXiv 1301.1493)、
  Anders・Schweitzer "Parallel computation of combinatorial symmetries" (arXiv 2108.04590)、
  Anders・Schweitzer・Stieß "Engineering a preprocessor for symmetry detection" (arXiv 2302.06351)。
  "Search problems in trees with symmetries" と bliss・saucy の原論文は読んでいない (以下の bliss・saucy の性質は上の 3 本の記述による)。
- 「事実」は出典 (ソース・LICENSE・論文) で確かめたもの、「推測」「未確認」はそう明記する。

## (1) 一覧表

| ソフト | 算法の要点 | 正準形 | 得意 / 苦手 (論文の記述) | ライセンス・版 | ENOMOTO に静的に結び付けて配れるか | 言語・ビルド | Rust のクレート (crates.io, 2026-11-10) | 使うソルバー |
|---|---|---|---|---|---|---|---|---|
| nauty | 個別化・細分化 (IR) の木を**深さ優先**。最初の葉 ν1 と (正準形を求めるときは) 不変量最大の葉 ν* を保つ。不変量による剪定 + 自己同型による剪定、群は random Schreier で保つ。密 (`densenauty`) と疎 (`sparsenauty`) の 2 つの表現 | 出す | 小さいグラフの大量処理に向く (1301.1493 の結論)。大きな疎の実用グラフは前処理なしでは遅い (2302.06351 図 8: sassy で劇的に速くなる) | Apache-2.0 (2.6r3 以降)。最新は 2.9.3 (クレート同梱の原本で確認) | 可 (Apache-2.0: 告知と LICENSE を同梱すればよい) | C。原本は `configure` + make。`nauty.h` などは configure が生成 | `nauty-Traces-sys` 0.11.0 (DL 30,349、Apache-2.0、nauty 2.9.3 を同梱し `cc` でビルド、既定 feature `bundled`)、`nauty-pet` 0.15.0 (DL 28,711、Apache-2.0、petgraph 向けの高水準)、`canonaut` 1.1.0 (DL 955、Apache-2.0、Rust への移植だが `DenseGraph` (ビット行列) のみ)、`graph-canon` 0.1.4 (DL 6,835、MIT、2023 で止まっている)、`umol-nauty-sys` 0.10.0 (DL 384) | SCIP 9.1 以降の既定 (`snauty` = sassy + nauty)。SCIP 9.0 でも選べる |
| Traces | IR の木を**幅優先** (各段で不変量最大の節点だけ残すので後戻りがいらず、不変量による剪定が最大に効く)。各節点から無作為の「実験の道」を葉まで下ろして自己同型を見つけ、random Schreier で群を保つ。次数 0/1/2/n−1 の頂点は個別化せず専用の処理 | 出す | 難しい組合せのグラフ (多くの難しいクラスで最速、1301.1493)。大きな実用グラフは不利: sassy 論文の MIP2017 で Traces 単体 803.6 秒 (saucy 22.3 秒、sy+Traces 92.8 秒)。巨大な SAT グラフでメモリ不足あり | nauty と同じ (Apache-2.0、`traces.c` は Piperno の著作) | 可 | C (nauty の配布物の一部) | nauty と同じクレート (`nauty-Traces-sys` に `Traces()` が入る) | SCIP の `compute_symmetry_sassy_nauty.cpp` に `#define NAUTY` を外すと Traces になる切り替えがある (既定は nauty) |
| bliss | IR の木を深さ優先 (nauty と同系)。成分ごとの再帰 (component recursion)、部分的な証明書による剪定 (HiGHS も「bliss の prefix pruning」を真似ている) | 出す | 互いに素・ほぼ互いに素な成分からなるグラフに専用の処理がある (1301.1493 の結論)。大きな実用グラフは sassy を前に置くと劇的に速い (2302.06351) | **LGPL-3.0** (`scipopt/bliss` の `COPYING.LESSER`)、0.77 (CMakeLists の VERSION) | 静的は実質不可 (LGPL-3 は利用者が差し替えて再リンクできることを求める。wheel に静的に焼くならオブジェクトの提供などが要る)。動的リンク (libbliss.so を別に置く) なら可 | C++。CMake か `Makefile-manual`。GMP は任意 | **なし** (`bliss-rs`・`bliss` は音楽解析・別物。`bliss-sys` は存在しない) → FFI を自作 | SCIP 8 の既定 (`SYM=bliss`)、SCIP 9.0 の既定 (`sbliss` = sassy + bliss)。BreakID も bliss を使う |
| saucy | IR の木を深さ優先 + 「疎な自己同型」を木の上の方で早く見つける (部分写像を恒等で延ばして確かめる)。疎なデータ構造で生成元の大きさに比例する手間 | 出さない (3.0 で bliss との合成で正準形を出せるとあるが未検証、1301.1493) | 大きな実用グラフ (SAT・MIP・道路網) で前処理なしの最速 (2302.06351)。動く頂点の少ない自己同型が多いグラフに強い。難しい組合せのグラフは弱い (pp16 で dejavu に大差、2302.06351 図 1) | **独自 (非商用のみ)**: "for NON-COMMERCIAL PURPOSES" (Univ. of Michigan, 2008)。版は 3.0 (2012) が最終と思われる (公式に届かず、版は未確認) | 不可 (非商用の制限。ENOMOTO の配布条件 (MIT) と両立しない) | C (`saucy.c`/`saucy.h` を落とし込むだけ。単体のツールは zlib が要る) | **なし** (`saucy` は Python 仮想環境の道具で別物) → FFI を自作 | 2302.06351 の比較相手。OR-Tools の自前の実装は saucy の論文 (Darga・Sakallah・Markov 2008) に基づく |
| dejavu | **無作為の IR の道** (根から葉への無作為歩行) で自己同型を集め、(random) Schreier-Sims の篩で群が揃ったかを確率的に判定 (片側誤差: 出す置換は必ず自己同型、見落としの確率 ≤ 1/2^10 が既定、`set_error_bound` で変更)。2.x では sassy の前処理・inprocessing・深さ優先・幅優先・無作為を組み合わせる (`dejavu.h` の `termination_strategy { t_prep, t_inproc, t_dfs, t_bfs, t_det_schreier, t_rand_schreier, t_limit }`)。終了が決定的だったかを `get_deterministic_termination()` で返す | 出さない (`dejavu.h` に正準形の API なし。自己同型群のみ) | 1 スレッドで Traces と競い、8 スレッドで多くのクラスで最速 (2108.04590。ただし現行 2.1 の公開版はスレッドを使うかは未確認)。sy+dejavu は sassy 論文の大きな実用グラフで最速 (MIP2017 で 14.07 秒)。苦手: Kronecker eye flip グラフ、巨大な SAT グラフでメモリ不足 | **MIT** (Copyright 2025 Markus Anders)、2.1 (`DEJAVU_VERSION_MAJOR 2`/`MINOR 1`) | 可 (MIT) | C++14、**ヘッダーだけ** (`#include "dejavu.h"`)。単体ツールは CMake | **なし** → FFI を自作 (C++ の薄い `extern "C"` の shim を `cc` で作る) | SCIP 10 で選べる (`SYM=dejavu`)。SCIP 10 は sassy の部分として dejavu 2.1 を同梱 (`DEJAVU_SHIPPED_VERSION 2.1`) |
| sassy | **前処理器** (単体では群を求めない)。色の細分化、次数 0/1/2 の頂点の除去、疎な自己同型の探り (probing)、商グラフの利用 (辺の反転・自明な成分の除去・連結成分)。縮めたグラフを後ろの解法 (nauty/Traces/bliss/saucy/dejavu) に渡し、見つかった置換を元のグラフに戻す hook を用意する | 出さない | 大きく疎な実用グラフを縮める。組合せのグラフでは何もしないが、上乗せの手間は無視できる (2302.06351)。MIP2017 で sy+dejavu 14.07 / sy+saucy 15.52 / sy+nauty 26.54 / sy+bliss 28.59 / saucy 単体 22.3 / Traces 単体 803.6 秒 (60 秒打ち切り、合計) | **MIT**。単独の repo (`markusa4/sassy`, 2022) と、dejavu 2.1 の `preprocess.h` (`SASSY_VERSION 2.1`) | 可 (MIT) | C++ (単独版は C++17、dejavu 内は C++14)、ヘッダーだけ。nauty/bliss/Traces/saucy への変換器 (`tools/*_converter.h`) つき | **なし** (`sassy` は文字列照合の別物) → dejavu と同じ shim で使える | SCIP 9 以降の既定の前段 (`sbliss`/`snauty`)。SCIP 10 は dejavu 同梱版を使う |

### ソルバーごとの使い方 (まとめ)

| ソルバー | 検出器 | 確かめた出典 |
|---|---|---|
| SCIP 8.0 | bliss (`SYM` の既定 `bliss`、選択肢は `bliss none`) | `scipopt/scip` の tag `v800`/`v801` の `CMakeLists.txt` |
| SCIP 9.0 | 既定 `sbliss` (sassy + bliss)。選択肢 `bliss sbliss nauty snauty none` | tag `v900`/`v901` |
| SCIP 9.1・9.2 | 既定 `snauty` (sassy + nauty) | tag `v910`/`v920` |
| SCIP 10.0 (10.0.0〜10.0.2) と master (11.0.0 開発版) | 既定 `snauty`。選択肢に `dejavu` が加わる (`bliss sbliss nauty snauty dejavu none`)。nauty 2.8.8 と dejavu 2.1 (sassy を含む) を同梱、bliss は外部 (`find_package(Bliss)`) | tag `v10.0.0`/`v10.0.2`、master の `CMakeLists.txt` (`NAUTY_SHIPPED_VERSION 2.8.8`, `DEJAVU_SHIPPED_VERSION 2.1`)、`src/symmetry/compute_symmetry_sassy_nauty.cpp` (`sassy.reduce` → `sparsenauty`、`symmetries/nautymaxlevel` で深さを制限、早期終了の hook あり) |
| HiGHS | 自前の `HighsSymmetryDetection` (MIT)。後戻りのある深さ優先の IR 探索、「bliss のような prefix pruning」、最初の葉・最良の葉の比較、ハッシュの衝突時の後戻り | `ERGO-Code/HiGHS` master の `highs/presolve/HighsSymmetry.{h,cpp}` (858 行目 "employ prefix pruning scheme as in bliss"、`backtrack`/`switchToNextNode`) |
| OR-Tools CP-SAT | 自前の `GraphSymmetryFinder` (Apache-2.0)。"largely based on" Darga・Sakallah・Markov 2008 (saucy の論文)。不変量の dive・本探索・軌道による剪定。Traces などの外部ライブラリは使っていない | `google/or-tools` main の `ortools/algorithms/find_graph_symmetries.h`、`ortools/sat/cp_model_symmetries.cc` (これを include)、`sat_parameters.proto` の `symmetry_level` (既定 2) |
| Gurobi・CPLEX | 非公開 | — |

## (2) ソフトごとの補足

### nauty
- McKay の古典。深さ優先の IR 木、不変量による剪定 (PA, PB) と自己同型による剪定 (PC)、random Schreier で群を保つ。
  正準形と自己同型群の両方。疎なグラフ用の `sparsenauty` があり、SCIP はこれを使う。
- ライセンス: Apache-2.0 (2.6r3 以降。`traces.c` は Piperno、`nautycliquer.c` などは別の著作者だが同じ Apache-2.0)。
  出典: クレート `nauty-Traces-sys` 0.11.0 同梱の `src/nauty2_9_3/COPYRIGHT`、`https://raw.githubusercontent.com/scipopt/scip/master/src/nauty/COPYRIGHT`。
- 版: 2.9.3 (`This_is_nauty2_9_3.txt`、`changes24-29.txt` の "Now 2.9.3")。公式 (pallini) には届かず、さらに新しい版があるかは未確認。
- 論文: https://arxiv.org/abs/1301.1493 (2 節・3.4 節: 深さ優先、"nauty is still preferred for mass testing of small graphs")。
- Rust: `nauty-Traces-sys` (https://crates.io/crates/nauty-Traces-sys) は原本を同梱して `cc` でビルドするので、利用者に C コンパイラがあれば
  `configure` は要らない。`SparseGraph` の補助、`optionsblk::default_sparse()` などの既定値がある。
  `canonaut` (https://github.com/um-univie/canonaut) は純 Rust の移植だが密なビット行列 (`DenseGraph`) だけで、数万〜数百万頂点の疎な MIP のグラフには向かない (README から判断。推測を含む)。

### Traces
- nauty と同じ配布物。幅優先で各段の不変量最大の節点だけを展開 (後戻りなし)、自己同型は無作為の「実験の道」の葉の比較で見つける。
  自己同型群だけを求めるときは、ある段で離散の子を見つけたら他の節点の離散の子と照合する別の戦略をとる (1301.1493 3.5 節)。
- 次数 0/1/2 の頂点は個別化しない。著者は「本来は前処理でやるべき」と書いており (3.6 節)、これが sassy の動機の一つ。
- sassy 論文では、大きな実用グラフ (特に MIP2017) で Traces は遅い (803.6 秒、sassy を前に置いても 92.8 秒) ので、MIP の用途には nauty の方が合う。
  SCIP も既定は nauty (`#define NAUTY`)。
- 出典: https://arxiv.org/abs/1301.1493、https://arxiv.org/abs/2302.06351 (図 9)。

### bliss
- Junttila・Kaski。nauty 系の深さ優先 IR に、成分の再帰と証明書による早い剪定を加えたもの。正準形を出す。
- ライセンス: LGPL-3.0、版 0.77 (https://raw.githubusercontent.com/scipopt/bliss/master/COPYING.LESSER、同 `CMakeLists.txt`)。
  `scipopt/bliss` は「SCIP 用の修正を入れた fork」(README)。公式 (users.aalto.fi) には届かず、最新版かは未確認。
- LGPL のため、ENOMOTO の wheel (cdylib) に静的に焼き込むと、利用者が bliss を差し替えて再リンクできる手段 (オブジェクトの提供など) を
  用意する義務が生じる。動的リンクで libbliss を別ファイルとして同梱するのが無難。いずれにせよ MIT の ENOMOTO とは配布の手間が増える。
- Rust のクレートはない (`bliss-rs` は音楽解析のライブラリで無関係)。C++ API (`bliss::Graph`) を包む `extern "C"` の shim が要る。

### saucy
- Darga・Liffiton・Sakallah・Markov。疎な実用グラフ向けで、木の上の方で疎な自己同型を見つけるのが要 (1301.1493 3.5 節)。
  正準形は出さない。
- ライセンス: "PERMISSION IS GRANTED ... FOR NON-COMMERCIAL PURPOSES" (Univ. of Michigan, 2008)。
  出典: https://raw.githubusercontent.com/andandandand/saucy-repo/master/LICENSE (saucy 2.1)、
  https://raw.githubusercontent.com/hrbrmstr/saucy/master/inst/COPYRIGHTS (saucy 3 系を包んだ R パッケージ)。
  公式サイトに届かないので 3.0 の LICENSE 原本は未確認 (R パッケージの COPYRIGHTS は同じ非商用の文言)。
- 非商用の制限があるので、ENOMOTO から配るのは不可 (商用利用を妨げない MIT の配布物に含められない)。
- 前処理なしでは大きな実用グラフで最速だったが、sassy を前に置いた他の解法 (sy+dejavu, sy+nauty) に追い越された (2302.06351 9.2 節)。

### dejavu
- Anders・Schweitzer。無作為の根→葉の歩行で自己同型を集め、Schreier-Sims の篩で「群が揃った」を確率的に判定。
  誤差は片側 (出力は必ず自己同型、見落としのみ)。著者いわく「対称性を利用する応用にはちょうどよい種類の誤り (見落としは遅くなるだけ)」。
  出典: https://arxiv.org/abs/2108.04590 (1 節、3 節)。
- 現行 2.1 は sassy の前処理 (`preprocess.h`)・inprocessing・深さ優先・幅優先・無作為の Schreier を組み合わせ、
  `set_error_bound` (既定 10 → 誤差 ≤ 1/2^10)、`set_prefer_dfs`、`set_limit_budget`、`set_seed`、`get_deterministic_termination()` などを持つ。
  正準形の API はない。出典: https://raw.githubusercontent.com/markusa4/dejavu/main/dejavu.h。
- ライセンス: MIT (https://raw.githubusercontent.com/markusa4/dejavu/main/LICENSE)。ヘッダーだけ、C++14 (README)。
- 生成元の通知は `(n, p, nsupp, supp)` の hook で、動いた点 (台) だけを渡すので大きなグラフでも置換ごとの手間が台の大きさで済む。

### sassy
- Anders・Schweitzer・Stieß 2023。単体では群を求めない前処理器。次数 0/1/2 の除去、疎な自己同型の探り、商グラフによる縮約などで
  大きく疎な部分を縮め、後ろの解法に渡す。見つかった置換を元のグラフへ戻す hook (`nauty_hook`, `traces_hook`, `bliss` 用の変換器) を持つ。
- 「bliss・nauty・dejavu は前処理で劇的に速くなる、Traces も速くなる、saucy ははっきりしない」「組合せのグラフでの上乗せの手間は無視できる」(9.1 節)。
  MIP2017 (MIP のグラフ、Pfetsch・Hojny 提供) の結果は表のとおり。
- ライセンス: MIT (https://raw.githubusercontent.com/markusa4/sassy/main/LICENSE)。現在は dejavu 2.1 の一部としても配布 (`preprocess.h` の
  "This file is part of dejavu 2.1"、`SASSY_VERSION 2.1`)。SCIP 10 も dejavu 同梱版の `dejavu::preprocessor` を使っている。
- 論文: https://arxiv.org/abs/2302.06351。

### グラフの形について (ENOMOTO に関係する点)
- 上のどれも入力は「頂点に色のある単純グラフ」で、**辺の重み (係数) は直接扱えない**。SCIP は辺の色 (係数) ごとに中間の頂点を挟んで
  符号化している (`src/symmetry/build_dejavu_graph.cpp`: 同じ頂点から出る同じ色の辺を一つの中間頂点にまとめる)。
  ENOMOTO から使う場合もこの変換 (と係数の色分け) を自作する必要がある。
- 自前の実装 (HiGHS・OR-Tools) は辺の重みをグラフに直接持たせられる点で外部ライブラリより素直。

## (3) ENOMOTO から使うならどれが現実的か (所見)

1. **配れるのは nauty/Traces (Apache-2.0)、dejavu/sassy (MIT)** の 2 系統だけ。bliss (LGPL-3.0) は動的リンクにしない限り配布が面倒、
   saucy (非商用) は不可。
2. **第一候補: dejavu 2.1 (sassy 込み)**。
   - ライセンスは MIT で ENOMOTO と同じ。ヘッダーだけなので、ファイルを vendoring して `cc` クレート (`cpp(true)`, `-std=c++14 -O3 -DNDEBUG`) で
     小さな `extern "C"` の shim を 1 本作れば済む。CMake も configure も要らない。
   - sassy 論文の MIP2017 で最速 (sy+dejavu 14.07 秒、SCIP の既定と同じ sy+nauty は 26.54 秒)。
   - 誤差は片側 (見落としだけ) で、ENOMOTO は今も得た置換を問題全体で確かめてから生成元にしているので、確率的であることは害にならない。
   - 手間の見積り (推測): shim (グラフの組み立て・hook で生成元を Rust の `Vec` に集める・誤差/予算の設定) が C++ 100〜200 行、
     Rust 側の FFI と、係数を色に写して中間頂点で符号化する変換が 300〜500 行。テストを含め 2〜4 日。
     Windows/macOS の wheel でも C++ コンパイラがあればよい (maturin の CI に C++14 が要る)。
3. **第二候補: nauty (`nauty-Traces-sys` 経由)**。
   - クレートがあるので FFI の手間が最小 (C の shim も要らない。`sparsenauty` に `userautomproc` として `extern "C" fn` を渡す)。
     手間の見積り (推測): グラフの変換を含め 1〜2 日。
   - ただし sassy なしの nauty は大きな実用グラフで大きく遅くなる (2302.06351 図 8)。SCIP と同じ構成 (sassy + nauty) にするには結局
     dejavu の `preprocess.h` を C++ の shim で呼ぶ必要があり、それなら dejavu 単体の方が簡単で速い。
   - `nauty-Traces-sys` のメンテナは 1 人 (a-maier)。GitHub の repo は raw で取れず (移転か非公開)、ソースはクレートの中身で確認した。
4. **価値の判断**: 今の ENOMOTO の検出は「後戻りなしの貪欲な 1 本の道」なので、個別化の選択を誤ると生成元を取りこぼす
   (`src/mip/symmetry.rs` の 8 行目・771〜773 行目のコメント。後戻りのある形 `search_bt` は `ENOMOTO_MIP_HO3_SYMBT` の試作として既にあり、既定では使わない)。
   外部ライブラリはここを完全な探索 (nauty) か確率保証つきの探索 (dejavu) に置き換えられ、さらに sassy の前処理で大きな MIP のグラフでも速い。
   HiGHS 並みの後戻り・剪定を自前で仕上げる手間 (HiGHS の `HighsSymmetry.cpp` は 1,936 行) と比べると、
   dejavu の vendoring + shim は安い。一方、C++ のビルド依存が wheel に入ること、辺の重みを中間頂点に符号化する変換が要ること、
   検出時間の上限 (dejavu の `set_limit_budget`、nauty の `usernodeproc` による打ち切り) を自前で設定する必要があることが代償。
   まず dejavu を feature フラグの後ろに入れて、既存の検出と生成元の数・時間を MIPLIB の対称な問題で比べるのがよい (推測に基づく提案)。
