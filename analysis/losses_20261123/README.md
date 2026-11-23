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
- 作った版 (この環境、`dist/build.txt`):
  - commit df0d3c6b3、rustc 1.97.0 (2d8144b78 2026-07-07)、maturin 1.16.0、profdata の sha1 ad8d3f9b5fbb968bdeebc12ee71797c02a139f9b
  - **.so の sha1 75f84cd62883f6452b4f9f638a1cf1b24cdcfbb7** (wheel enomoto_solver-0.1.10-cp313-cp313-manylinux_2_39_x86_64.whl)
  - 学習から作って 15 分弱 (pgogen と pgo_v3 で各 6〜7 分、学習 2 分)。コミット済みの学習結果から pgo_v3 だけ作ると 7 分ほど。
  - コミット済みの学習結果から作り直すと、.so の sha1 が同じになった (同じコミット・同じ rustc なら同じ .so)。wheel の sha1 は zip の時刻で変わるので、
    照合は .so の sha1 で行う。
- 各分担の手順 (同じ .so で測るため):
  1. `git checkout df0d3c6b3` 以降のこの台本のコミットで (build.sh は HEAD を作るので、**ソースが df0d3c6b3 と同じ**コミットで作ること)
  2. `rustup component add llvm-tools` は学習から作るときだけ要る。学習結果から作るなら不要。
  3. `pip install maturin highspy==1.15.1 pyscipopt psutil`
  4. `PROF=analysis/losses_20261123/pgo/merged.profdata.gz sh analysis/losses_20261123/build.sh` → 最後に出る so_sha1 が 75f84cd6… と同じことを確かめる
     (違えば rustc の版かソースが違う。その場合も dist/build.txt を結果と一緒に残せば、行の ours_so_sha1 でどの版か分かる)
  5. `pip install --force-reinstall --no-deps analysis/losses_20261123/dist/enomoto_solver-*.whl`
  - 1 本だけ作って配る形にするなら、上の 4 で作った wheel を各分担の機械に写して 5 だけをする (同じ Python の版 (cp313) と glibc 2.39 以上が要る)。

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
