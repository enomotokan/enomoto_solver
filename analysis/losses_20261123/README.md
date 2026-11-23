# HiGHS・SCIP に負けている問題の測り直し (2026-11-23、利用者の依頼、台本と分担表)

これまでの比べで相手に 1.5 倍以上負けた (または相手だけ解けた) 問題を、今の分岐の先頭の既定を PGO + x86-64-v3 で作った当方と、
SCIP・HiGHS で 600 秒・種 1 で測り直す。台本は analysis/holdout_20261123 の run.py・summ.py の写し (同じ worker・ギャップ 0・種の渡し方)。

## 1. 問題 (pick.py → picked.json、48 問)

出どころ (当方 / 相手の時間の比、shift 1 秒、解けない回は上限で数え、種ごとの比の幾何平均):
- vsscip_20261120 の M (今の既定 def (4 スレッド)・def_t1 (1 スレッド) / 11/17 の scip)
- vsscip_20261117 の P (def・def_t1 / scip、60 秒、種 1〜3)
- vshighs_20261117 の MA (M・A の Mittelmann 186 問、600 秒) と P (def / highs、どちらも 4 スレッド、0.1.9)
- 拾う条件: 比が 1.5 以上、または相手だけが解けた回がある。さらに comp07-2idx (RINS_PAR の M で既定でも時間のかかった問題) を足した。
- 目安 30〜45 問に対して 48 問になった。条件で拾ったものなので削っていない。vshighs (0.1.9) だけで拾われた問題には、今の既定では差が縮んだものもありうる。

| 問題 | 出どころと比 (当方 / 相手) |
|---|---|
| neos859080 | vsscip1120 M def/scip 4.57; vsscip1120 M def_t1/scip 2.22; vshighs1117 MA def/highs 109.67 |
| neos-1171448 | vsscip1120 M def/scip 2.25; vsscip1120 M def_t1/scip 27.5 |
| graph20-20-1rand | vsscip1120 M def_t1/scip 1.9; vsscip1117 P def/scip 8.92 (相手だけ解けた 3/3); vsscip1117 P def_t1/scip 8.92 (相手だけ解けた 3/3) |
| CMS750_4 | vsscip1120 M def/scip 1.64; vsscip1120 M def_t1/scip 7.31 (相手だけ解けた 1/1) |
| fhnw-binpack4-4 | vsscip1120 M def/scip 5.85; vsscip1120 M def_t1/scip 4.7 |
| istanbul-no-cutoff | vsscip1120 M def/scip 2.42; vsscip1120 M def_t1/scip 5.39 (相手だけ解けた 1/1) |
| decomp2 | vsscip1120 M def_t1/scip 4.52 |
| bnatt400 | vsscip1120 M def/scip 1.76; vsscip1120 M def_t1/scip 4.21 (相手だけ解けた 1/1) |
| swath1 | vsscip1120 M def_t1/scip 3.55; vsscip1117 P def_t1/scip 2.08 (相手だけ解けた 1/3) |
| 10teams | vsscip1117 P def_t1/scip 3.53 (相手だけ解けた 1/3); vshighs1117 P def/highs 2.64 |
| mzzv11 | vshighs1117 MA def/highs 3.23 |
| hypothyroid-k1 | vsscip1117 P def/scip 3.14; vsscip1117 P def_t1/scip 2.21; vshighs1117 P def/highs 2.03 |
| seymour1 | vsscip1120 M def/scip 1.64; vsscip1120 M def_t1/scip 2.95 |
| neos8 | vsscip1120 M def/scip 2.01; vsscip1120 M def_t1/scip 2.31; vsscip1117 P def/scip 2.12; vsscip1117 P def_t1/scip 2.7 |
| rail507 | vsscip1120 M def_t1/scip 2.7 (相手だけ解けた 1/1) |
| csched008 | vshighs1117 MA def/highs 2.67 (相手だけ解けた 2/2) |
| qnet1 | vshighs1117 P def/highs 2.49 |
| mc11 | vsscip1120 M def_t1/scip 1.85; vsscip1117 P def_t1/scip 2.28 (相手だけ解けた 1/3) |
| p200x1188c | vshighs1117 MA def/highs 2.1; vshighs1117 P def/highs 2.22 |
| dano3_3 | vshighs1117 MA def/highs 2.19 |
| mcsched | vsscip1120 M def_t1/scip 2.14 (相手だけ解けた 1/1) |
| dano3_5 | vsscip1120 M def_t1/scip 2.1 (相手だけ解けた 1/1) |
| mzzv42z | vshighs1117 MA def/highs 2.09; vshighs1117 P def/highs 1.04 (相手だけ解けた 2/2) |
| rocI-4-11 | vsscip1120 M def_t1/scip 2.05 |
| tbfp-network | vsscip1120 M def_t1/scip 1.07 (相手だけ解けた 1/1); vshighs1117 MA def/highs 2.04 (相手だけ解けた 1/2) |
| sp150x300d | vshighs1117 MA def/highs 2.03; vshighs1117 P def/highs 1.81 |
| 30n20b8 | vsscip1120 M def_t1/scip 1.95 |
| noswot | vsscip1117 P def/scip 1.2 (相手だけ解けた 1/3); vsscip1117 P def_t1/scip 1.95 (相手だけ解けた 2/3) |
| ns1830653 | vsscip1120 M def_t1/scip 1.9 |
| khb05250 | vshighs1117 P def/highs 1.83 |
| neos-3381206-awhea | vsscip1120 M def/scip 1.62; vsscip1120 M def_t1/scip 1.81 |
| physiciansched6-2 | vshighs1117 MA def/highs 1.81; vshighs1117 P def/highs 1.63 |
| air03 | vsscip1117 P def_t1/scip 1.8 |
| pg | vshighs1117 MA def/highs 1.78 |
| neos-873061 | vshighs1117 MA def/highs 1.76 (相手だけ解けた 2/2) |
| irp | vsscip1120 M def_t1/scip 1.71 |
| exp-1-500-5-5 | vsscip1117 P def/scip 1.67; vsscip1117 P def_t1/scip 1.63 |
| misc07 | vsscip1117 P def_t1/scip 1.67 |
| h80x6320d | vshighs1117 P def/highs 1.54 |
| satellites2-60-fs | vshighs1117 MA def/highs 1.48 (相手だけ解けた 2/2) |
| net12 | vshighs1117 MA def/highs 1.47 (相手だけ解けた 2/2) |
| sct2 | vshighs1117 MA def/highs 1.21 (相手だけ解けた 2/2) |
| uccase12 | vshighs1117 MA def/highs 1.12 (相手だけ解けた 1/1) |
| neos-3216931-puriri | vshighs1117 MA def/highs 1.11 (相手だけ解けた 1/1) |
| traininstance6 | vshighs1117 MA def/highs 1.09 (相手だけ解けた 2/2) |
| satellites2-40 | vshighs1117 MA def/highs 1.01 (相手だけ解けた 1/1) |
| neos-933966 | vshighs1117 MA def/highs 0.9 (相手だけ解けた 1/2) |
| comp07-2idx | RINS_PAR の M (既定でも時間がかかった) None |

- MIPLIB の既知の値 (miplib2017-v31.solu) を solu.json に入れた (48 問すべてにある)。集計で最適値・実行不能の食い違いを見る。

## 2. 設定 (run.py の CFGS、holdout と同じ)

ours_t1・ours_t4 (当方の既定、ENOMOTO_ の環境変数は全部外す、1・4 スレッド)、scip (pyscipopt 6.3.0 = SCIP 10.0.2、逐次、limits/gap = 0、
種は randomseedshift)、highs_t1・highs_t4 (highspy 1.15.1、mip_rel_gap = 0、threads 1・4、種は random_seed)。600 秒 (P の問題も)、種 1。
1 回ずつ別プロセス、同時に 1 本だけ。行には当方・highspy・pyscipopt の版と、読み込んだ当方の .so の sha1 (`ours_so_sha1`) を残す。

## 3. 当方のビルド (PGO + x86-64-v3、build.sh)

analysis/buildopt_20261122 の pgo_v3 と同じ作り方 (LTO fat・codegen-units 1・`-C target-cpu=x86-64-v3`・`-Cprofile-use`)。Cargo.toml は変えず、
環境変数だけで替える。`build.sh` は今の HEAD を git worktree で別に取り出して作る (作業ツリーの変更は入らない)。

- 学習 (PGO): 測る 48 問と重ならない P・M の 10 問 (neos17, binkar10_1, eil33-2, neos-827175, nu25-pr12, dcmulti, qiu, rout, mas76, neos-911970) を、
  学習用の版 (pgogen) で 1 スレッド・種 1・上限 30 秒で解く。buildopt の学習の 10 問のうち 4 問 (graph20-20-1rand・hypothyroid-k1・p200x1188c・misc07)
  は今回の 48 問に入るので入れ替えた。学習の結果は `pgo/merged.profdata.gz` (2.5 MB) としてコミットした。
- **.so の sha1 は作るたびに変わる** (2026-11-23 に分かった、親の指摘で調べた):
  - 同じコミット (40be041e4)・rustc 1.97.0・学習結果 (sha1 ad8d3f9b…)・フラグで作っても、分担 0 は fd9e4a30…、分担 2 は 3432a101…、分担 5 は c988d5f2…、
    この環境の作り直しは 38eea0d8… になった (最初に書いた 75f84cd6… は、依存を作り直さない 2 回目と一致しただけだった)。
  - .so には依存クレートのソースの絶対パス (CARGO_HOME/registry/src/…、121 か所) が文字列として入る。RUSTFLAGS には学習結果のパス
    (`-Cprofile-use=<リポジトリ>/analysis/losses_20261123/target/merged.profdata`) が入り、これだけでも .so が変わった。
    `--remap-path-prefix` で CARGO_HOME の場所を消しても、それを足した RUSTFLAGS の違いで .so は変わった。cargo の trim-paths は 1.97 では使えない。
  - なので **sha1 では照合しない**。同じ版の条件は、dist/build.txt の commit・rustc・maturin・profdata の sha1・env がそろうこと (各分担は build_sK.txt に残す)。
  - 動きの照合: `python analysis/losses_20261123/check_build.py` (1 スレッド・決定的・種 1 で khb05250・air03・neos8 を解き、状態・目的値・ノード数を
    ref_check.json と比べる)。作り方 (LTO・PGO・target-cpu) は buildopt で道筋を変えないと確かめてあるので、これはソースの照合になる。
  - 集計は .so の sha1 ごとの行の数を出し、`summ.py --so <先頭 8 文字>` で当方の行を 1 つの .so に絞れる。
- 各分担の手順 (同じ作り方で作るため):
  1. `git checkout 40be041e4` (この台本の最初のコミット。親が df0d3c6b3 でソースは同じ)。build.sh は HEAD を作るので、このコミットで作る。
     ブランチの先端は後から他の変更 (src/lib.rs・python/enomoto_solver/__init__.py) が入っているので、先端で作るとソースが変わる。
  2. `pip install maturin highspy==1.15.1 pyscipopt psutil`
  3. `PROF=analysis/losses_20261123/pgo/merged.profdata.gz sh analysis/losses_20261123/build.sh` (約 7 分) → dist/build.txt を build_sK.txt として残す
  4. `pip install --force-reinstall --no-deps analysis/losses_20261123/dist/enomoto_solver-*.whl`
  5. ブランチに戻り `python analysis/losses_20261123/check_build.py` が「一致」を出すことを確かめてから `run.py K`

### 3.1 PGO が当たっているかの確かめ (2026-11-23、親の依頼)

- 前の build.sh (学習用を x86-64-v3 なしで作り、作業木は mktemp の毎回違う場所) で計測用の版を作ると、LLVM が当方のクレートの関数 1281 個
  (std の汎用関数 1216 個も) の学習結果を「function control flow change detected (hash mismatch)」として捨てていた
  (`-Cllvm-args=-pgo-warn-missing-function` つきで作り直して数えた。捨てた中には単体法の solve_slope_intercept_dual_impl も入る)。
  記号名は合っていて、関数の形 (制御の流れのハッシュ) が違う: 学習用と計測用で target-cpu が違うためと見る。
- build.sh を直した (コミット b03029409 以降): 学習用も `-C target-cpu=x86-64-v3` で作り、作業木と学習結果を固定の場所 (/tmp/enomoto_losses_build) に
  置く。build.txt に hash mismatch の数 (`pgo_hash_mismatch`) を書く。学習から作り直すと 0、コミットした新しい学習結果から作り直しても 0。
  ただし .so の sha1 は固定の場所でも作るたびに変わった (777dabbf… と 5bb2b826…)。sha1 で照合しない方針は変えない。
- 時間の比べ (`pgo_check.py`、1 スレッド・決定的・種 1、学習と重ならない 6 問 × 2 回、速い方どうし、res_pgocheck.jsonl):

| 問題 | lto_v3 (PGO なし) [秒] | 前の版 (38eea0d8…) | 新しい版 (777dabbf…) | 前/lto_v3 | 新/lto_v3 |
|---|---:|---:|---:|---:|---:|
| air03 | 4.05 | 3.89 | 4.20 | 0.961 | 1.038 |
| neos8 | 4.77 | 4.66 | 4.25 | 0.976 | 0.890 |
| exp-1-500-5-5 | 4.79 | 4.53 | 4.65 | 0.945 | 0.971 |
| qnet1 | 3.16 | 3.01 | 3.21 | 0.953 | 1.016 |
| mik-250-20-75-4 | 10.28 | 10.35 | 10.26 | 1.007 | 0.998 |
| markshare_4_0 | 6.07 | 5.49 | 5.38 | 0.904 | 0.887 |
| **幾何平均** | | | | **0.957** | **0.965** |

  - 道筋 (状態・ノード数・目的値) は 3 版とも全問で同じ。
  - 前の版も新しい版も PGO なしより 4% ほど速い。2 つの差は揺れの内 (buildopt で同じ版の 2 回の揺れが 2〜4%)。
    1281 関数の学習結果を捨てていても、残りの関数 (依存クレート・当たった当方の関数) の分で効いていた。
  - **結論: 分担が前の build.sh で作った版でも、当方の行を取り直す必要はない**。次に作るときは直した build.sh を使う。

## 4. 分担と回し方 (plan.py → shards.json)

- 48 問 × 5 設定 × 種 1 = 240 回、**6 分担**、見積り各 3.46〜3.49 時間、計 20.8 時間 (1 分担は 2 時間弱 × 2 回)。
  - 見積り (1 回): ours_t1・ours_t4 は vsscip_20261120 の M の def_t1・def (P の問題は vsscip_20261117 の P で解けた回の時間、解けなければ 600)、
    scip は vsscip_20261117、highs_t4 は vshighs_20261117、highs_t1 は highs_t4 の 2 倍 (計測がない)。記録がなければ 600。それぞれ待ち 30 秒を足す。
  - 設定ごと (時間): ours_t1 4.6、ours_t4 3.2、scip 3.5、highs_t1 5.3、highs_t4 4.2。
  - 同じ問題の 5 設定は同じ分担で続けて解き、順は問題ごとに回して入れ替える。作り直しは `python plan.py 6`。
- 回し方: `python analysis/losses_20261123/run.py K` (K = 0〜5) → res_sK.jsonl。止めても同じコマンドで続きから回る (crash・取得の失敗の行はやり直す)。
- 集計: `python analysis/losses_20261123/summ.py > summ.txt` (状態の内訳、解けた数、最大 RSS、食い違い (設定どうし・MIPLIB の既知の値)、
  ours_t1 / scip・ours_t1 / highs_t1・ours_t4 / highs_t4 の SGM の比 (shift 1、問題単位のブートストラップ 95%)、2 倍以上負けた・勝った問題)。
  版の内訳 (ours_so_sha1 を含む) を始めに出すので、分担で違う .so を使えば分かる。

## 5. 台本の確かめ (この環境、結果はコミットしない)

- 上の wheel を入れて、khb05250・air03 を上限 10 秒で 5 設定 × 1 回: 10 行とも optimal、最適値は 5 設定で一致 (食い違い 0)、
  行の ours_so_sha1 は 75f84cd6…。もう一度回すと残り 0 回。summ.py も通った。

## 6. 結果 (2026-11-23、6 分担 240 行、summ.txt・compare.txt)

- 行 240 (48 問 × 5 設定 × 種 1、600 秒)。crash 0、killed 1 (satellites2-40 の scip が 600 + 300 秒で止まらず run.py が打ち切り)。
- 当方の .so は分担ごとに 1 つずつ (e2cf1200・f027665a・c8babad5・3432a101・fd9e4a30・cd2f35eb、各 16 行)。どれも 40be041e4 を前の build.sh で作った版
  (3.1 節: 版による時間の差は揺れの内)。分担 5 は check_build.py も「一致」。
- 食い違い 0 (設定どうし・MIPLIB の既知の値)。分担 3・5 の報告にあった 2 件ずつは、HiGHS が時間切れ・実行不能で目的値 inf を返したのを解ありと
  読んだ summ.py の誤りで、直した (9801554cb)。

### 6.1 解けた数と SGM

| 設定 | 解けた数 (48 問) |
|---|---:|
| ours_t1 | 33 |
| ours_t4 | 38 |
| scip | 37 |
| highs_t1 | 38 |
| highs_t4 | 38 |

| 比 (当方 / 相手) | SGM (shift 1 秒) | 95% 区間 | どちらかが解けた問題 | 当方だけ解けた / 相手だけ解けた |
|---|---:|---|---:|---|
| ours_t1 / scip | 1.296 | [0.879, 1.931] | 38 | 1 / 5 |
| ours_t1 / highs_t1 | 1.134 | [0.769, 1.632] | 42 | 4 / 9 |
| ours_t4 / highs_t4 | 0.710 | [0.497, 0.960] | 46 | 8 / 8 |

- 負けている問題を選んで測っているので、比は全体の比べではない (選び方で当方に不利に偏る)。
- 1 スレッドどうしではまだ負け越し (SCIP に 1.30 倍、HiGHS に 1.13 倍、どちらも区間は 1 をまたぐ)。4 スレッドどうしでは HiGHS に 0.71 倍 (区間も 1 未満)。

### 6.2 今も負けている問題 (どれかの組で 2 倍以上負け、または相手だけ解けた: 27 問)

比は当方 / 相手の時間 (shift 1 秒、時間切れは 600 秒)。括弧は前回の比 (picked.json の出どころ)。「相手だけ」は相手だけが解けた。動きは今回が前回の 1/1.25 未満なら縮んだ、1.25 倍超なら広がった。
右の 3 列は、当方 (1 スレッド) の根の下界・最初の解の時間・ノード数と、scip・highs_t4 の下界・ノード数 (時間切れなら最終の下界)。

| 問題 | ours_t1/scip (前回) | ours_t4/scip (前回) | ours_t1/highs_t1 (前回) | ours_t4/highs_t4 (前回) | 動き | 当方 t1: 根の下界・最初の解 (秒)・ノード | scip: 下界・ノード | highs_t4: 下界・ノード |
|---|---|---|---|---|---|---|---|---|
| 10teams | 16.00 (3.53) | 0.79 | 62.62 | 3.05 (2.64) | 同じくらい・広がった | 924・12.1263・15596 | 924・5 | 924・1 |
| 30n20b8 | 2.82 相手だけ (1.95) | 0.57 | 両方解けず | 0.20 | 広がった | 141.881・420.215・12248 | 302・2069 | 151・3151 |
| CMS750_4 | 14.40 相手だけ (7.31) | 5.90 (1.64) | 両方解けず | 0.41 | 広がった | 251・0.314892・31205 | 252・652 | 250・9629 |
| bnatt400 | 4.06 相手だけ (4.21) | 1.62 (1.76) | 両方解けず | 0.40 | 同じくらい | 0・558.528・12429 | 1・4098 | 0・15466 |
| csched008 | 両方解けず | 両方解けず | 2.00 相手だけ | 2.07 相手だけ (2.67) | 縮んだ | 171・—・72067 | 171.402・28814 | 173・18074 |
| dano3_5 | 2.41 (2.1) | 1.41 | 1.15 | 0.68 | 同じくらい | 576.289・19.7367・548 | 576.925・122 | 576.925・127 |
| fhnw-binpack4-4 | 5.85 (4.7) | 7.56 (5.85) | 0.09 | 0.12 | 同じくらい・広がった | 0・—・10977 | 1e+20・10500 | 0・484338 |
| istanbul-no-cutoff | 4.54 相手だけ (5.39) | 2.21 (2.42) | 両方解けず | 0.49 | 同じくらい | 70.9999・51.7043・1451 | 204.082・319 | 109.106・1473 |
| mcsched | 2.12 (2.14) | 0.49 | 1.45 | 0.36 | 同じくらい | 193787・0.670458・78143 | 211913・8068 | 211913・14726 |
| mzzv11 | 1.80 | 0.60 | 5.32 | 1.71 (3.23) | 縮んだ | -21855.9・11.6619・2428 | -21718・1615 | -21718・1 |
| neos-1171448 | 15.96 (27.5) | 1.26 (2.25) | 4.47 | 0.36 | 縮んだ | -309・5.6335・1203 | -309・1 | -309・148 |
| neos-3216931-puriri | 両方解けず | 両方解けず | 1.44 相手だけ | 1.53 相手だけ (1.11) | 広がった | 59024.1・290.127・6 | 59134.4・121 | 71320・336 |
| neos-873061 | 両方解けず | 両方解けず | 1.80 相手だけ | 1.86 相手だけ (1.76) | 同じくらい | 104.742・8.78023・14 | 103.798・1 | 113.656・7645 |
| neos8 | 2.33 (2.7) | 1.81 (2.12) | 1.20 | 1.04 | 同じくらい | -3719・5.86688・0 | -3719・1 | -3719・1 |
| neos859080 | 1.26 (2.22) | 3.29 (4.57) | 1.30 | 3.36 (109.67) | 縮んだ | 1・—・2561 | 1e+20・966 | —・2374 |
| net12 | 両方解けず | 両方解けず | 1.82 相手だけ | 1.87 相手だけ (1.47) | 広がった | 79.4766・1.42489・1428 | 121.41・986 | 214・980 |
| ns1830653 | 2.06 (1.9) | 0.69 | 1.27 | 0.42 | 同じくらい | 10497・7.37249・18035 | 20622・5114 | 20622・13250 |
| pg | 0.55 | 0.30 | 2.74 | 1.38 (1.78) | 縮んだ | -8680.32・0.0356586・1093 | -8674.34・548 | -8674.34・75 |
| qnet1 | 0.18 | 0.13 | 2.76 | 2.02 (2.49) | 同じくらい | 15942.8・0.0620839・3 | 16029.7・8 | 16029.7・1 |
| rocI-4-11 | 2.67 (2.05) | 1.05 | 0.69 | 0.27 | 広がった | -1.10705e+07・8.38132・23180 | -6.0202e+06・17120 | -1.10604e+07・33266 |
| satellites2-40 | 両方解けず | 両方解けず | 1.38 相手だけ | 1.44 相手だけ (1.01) | 広がった | -29・158.26・32 | —・— | -19・139 |
| satellites2-60-fs | 両方解けず | 両方解けず | 1.97 相手だけ | 1.94 相手だけ (1.48) | 広がった | -29・82.9323・252 | -29・1 | -19・141 |
| sct2 | 両方解けず | 両方解けず | 1.45 相手だけ | 1.49 相手だけ (1.21) | 同じくらい | -231.047・0.693933・24697 | -231.11・68514 | -230.989・20642 |
| seymour1 | 2.35 (2.95) | 1.55 (1.64) | 1.31 | 0.85 | 同じくらい・縮んだ | 405.882・2.69725・4971 | 410.764・1552 | 410.764・2153 |
| swath1 | 3.30 (3.55) | 1.24 | 2.04 | 0.85 | 同じくらい | 340.576・6.8267・4547 | 379.071・141 | 379.071・113 |
| tbfp-network | 1.05 相手だけ (1.07) | 0.39 | 1.93 相手だけ | 0.72 (2.04) | 同じくらい・縮んだ | 23.5672・6.05484・203 | 24.1632・35 | 24.1632・5 |
| traininstance6 | 両方解けず | 両方解けず | 1.99 相手だけ | 1.90 相手だけ (1.09) | 広がった | 0・6.15982・54004 | 731.255・14957 | 28290・115954 |

- 2 倍以上負けから外れた問題 (21 問): 当方が追いついた問題。decomp2 (0.1 秒、前処理で解ける)・hypothyroid-k1・graph20-20-1rand・noswot・misc07 などは逆に大きく勝つ。
- 広がったのは 10teams (1 スレッド、前回 3.53 → 16.0)・30n20b8・CMS750_4 (1 スレッド、時間切れ)・rocI-4-11・fhnw-binpack4-4 (4 スレッド / scip 5.85 → 7.56) と、
  HiGHS だけが解く 5 問 (net12・satellites2-40・satellites2-60-fs・traininstance6・neos-3216931-puriri)。この 5 問は前回の 4 スレッドの比が 1.0〜1.5 倍で、
  今回は当方が時間切れになり 1.4〜2.0 倍になった。

### 6.3 負けの型の見立て

当方の根の下界と相手の下界・最終の下界、最初の解の時間、ノード数、これまでの調べ (vsscip_slow_20261118 の各群、slowm_20261119、scipslow_20261119) から分けた。
1 回ずつの計測なので、運の分は残る。

| 型 | 問題 | 手がかり |
|---|---|---|
| 前処理・対称性 | 30n20b8、bnatt400、CMS750_4、fhnw-binpack4-4、traininstance6 | 30n20b8 は当方の根の下界 141.9 に対し SCIP は 302 (= 最適値) で、前処理で決まる。traininstance6 は根の下界 0 に対し HiGHS は 28290 (= 最適値)。bnatt400 は SCIP が前処理・伝播で決着する群 1。CMS750_4 は前処理の集約 (pcd) で縮む兆しがあった。fhnw-binpack4-4 は SCIP の対称性 (misc/usesymmetry) が決め手 (HANG の調べ) |
| 根の LP・小さい問題の手間 | neos8 | 根の下界 = 最適値、ノード 0 で 6.0 秒 (SCIP 2.0 秒)。根までの手間 |
| 根の解探し | 10teams、neos-1171448、csched008、neos-3216931-puriri、neos-873061 | 10teams・neos-1171448 は根の下界 = 最適値なのに、1 スレッドでは最適解が遅れて 1 万ノード前後 (SCIP は 1〜5 ノード、HiGHS は 1 ノード)。4 スレッドなら 3.6・17.8 秒。neos-1171448 は根の RENS が −308 止まり (slowm 1 節)。csched008 は当方 1 スレッドで解なし、neos-3216931-puriri・neos-873061 は暫定解が最適値から 14%・7% 悪い |
| カット (根の下界) | istanbul-no-cutoff、mcsched、mzzv11、net12、ns1830653、pg、qnet1、satellites2-40、satellites2-60-fs、sct2、seymour1、swath1、tbfp-network | どれも当方の根の下界が相手より弱い: istanbul 71 対 204 (SCIP)、mcsched 193787 対 211913、net12 79.5 対 214 (HiGHS)、ns1830653 10497 対 20622、swath1 340.6 対 379.1、qnet1 15942.8 対 16029.7、mzzv11 −21855.9 対 −21718 (HiGHS は 1 ノード)、satellites2 −29 対 −19。数が最も多い |
| 木 (分枝・衝突・LP の手間) | dano3_5、neos859080、rocI-4-11 | dano3_5 は根の下界が同じで、ノード 548 対 122 (LP の重い木)。neos859080 は実行不能の証明で SCIP の衝突解析 (conflict/useprop) が決め手 (HANG の調べ)。rocI-4-11 は 1 スレッドだけ遅い (4 スレッドは 161.5 秒で SCIP 並み) |

- 多いのはカット (根の下界) の 13 問で、次が根の解探し (5 問) と前処理・対称性 (5 問)。
- 1 スレッドだけ大きく負ける問題 (10teams・neos-1171448・CMS750_4・rocI-4-11) は、4 スレッドでは並ぶか勝つ。1 スレッドでは並列の解探し (Local-MIP・並列のサブ MIP) がない分、解探しの弱さがそのまま出ていると見る。
