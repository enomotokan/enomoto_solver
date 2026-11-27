# 調整に使っていない問題での他ソルバーとの比較 (2026-11-23、利用者が手元のパソコンで回す台本)

これまでの調整・採否の判定には、P (MIPLIB の 58 問、analysis/cbcimpl_20261029/probs.py) と M (Mittelmann の 186 問、
analysis/cbcimpl_20261029/mitt_parts.json) だけを使ってきた。調整に使っていない問題でも同じように SCIP・HiGHS と比べられるかを見るため、
MIPLIB 2017 の benchmark 集合 (240 問) から P・M に入る問題を除いた **53 問** を、当方・SCIP・HiGHS で解く。

## 1. 問題 (problems.json、53 問)

- 元: https://miplib.zib.de/downloads/benchmark-v2.test (240 問)。P と重なるのは 42 問、M と重なるのは 186 問 (M は全部 benchmark 集合に入る)、
  どちらかに入るのは 187 問。残り 53 問:

```
bab2 bab6 buildingenergy co-100 eilA101-2 ex10 ex9 gfd-schedulen180f7d50m30k18 highschool1-aigio irish-electricity
k1mushroom map10 map16715-04 neos-2075418-temuka neos-2987310-joes neos-3402294-bobin neos-3402454-bohle neos-3555904-turama
neos-4413714-turia neos-4532248-waihi neos-4647030-tutaki neos-5049753-cuanza neos-5052403-cygnet neos-5093327-huahum
neos-5104907-jarama neos-5114902-kasavu neos-631710 neos-848589 netdiversion ns1644855 ns1760995 nursesched-medium-hint03
physiciansched3-3 proteindesign121hz512p9 proteindesign122trx11p8 rail02 rd-rplusc-21 roi2alpha3n4 roi5alpha10n8 s100
s250r10 savsched1 splice1k1 square41 square47 supportcase10 supportcase12 supportcase19 supportcase22 supportcase6
supportcase7 triptim1 var-smallemery-m6j6
```

- MIPLIB の既知の値 (solu.json、https://miplib.zib.de/downloads/miplib2017-v31.solu から): 最適値 50 問、実行不能 2 問、最良の値 (最適は未証明) 1 問。
  集計で、最適値・実行不能の判定がこれと食い違えば誤答として出す。
- これらの問題は過去の数え上げ (analysis/drcr_20261110 など) や個別の調べには出てくるが、採否の判定 (P・M の計測) には使っていない。
- MPS の取り方: run.py が scripts/run_miplib_benchmark.py の `fetch` で https://miplib.zib.de/WebData/instances/<名前>.mps.gz から取り、
  展開して `.miplib_cache/` (`--cache` で変えられる) に置く (一度だけ)。53 問の gz は合計約 524 MB (最小 1.9 MB の triptim1、最大 82 MB の square47)、
  展開後はその数倍になるので、空きを数 GB 見ておく。

## 2. 設定 (run.py の CFGS)

| 名前 | ソルバー | スレッド | 主な設定 |
|---|---|---|---|
| ours_t1 | 当方の既定 (ENOMOTO_ で始まる環境変数は全部外す) | 1 | ENOMOTO_MIP_THREADS=1、種は ENOMOTO_T_MIP_SEED |
| ours_t4 | 同上 | 4 | ENOMOTO_MIP_THREADS=4 |
| scip | pyscipopt の SCIP (6.3.0 なら SCIP 10.0.2) | 1 (SCIP は既定で逐次) | limits/gap = 0、種は randomization/randomseedshift |
| highs_t1 | highspy の HiGHS (1.15.1) | 1 | mip_rel_gap = 0、threads = 1、種は random_seed |
| highs_t4 | 同上 | 4 | threads = 4 |

- どれも scripts/run_miplib_benchmark.py の worker (`--worker-ours`・`--worker-scip`・`--worker-highs`) で、1 回ずつ別のプロセスで解く。
  時間は読み込みを除いた求解の時間 (当方・HiGHS・SCIP とも同じ)。ギャップ 0・SCIP の種の渡し方は analysis/vsscip_20261117・vshighs_20261117 と同じ。
- HiGHS の種: vshighs_20261117 は種を渡していなかった (HiGHS の既定 random_seed = 0)。ここでは種を random_seed に渡す (種 1 つなら影響は小さい)。
- 同時に解くのは 1 本だけ (機械の揺れを避ける)。上限 + 300 秒を過ぎたら打ち切って `killed` とする。最大 RSS は 0.5 秒ごとに見る
  (psutil があればそれで。なければ Linux の /proc。macOS・Windows で RSS を取るには psutil が要る)。

## 3. 手元のパソコンでの準備

Python 3.9 以上。リポジトリを手元に置き (台本と scripts/run_miplib_benchmark.py を使う)、その根で:

```
# 仮想環境 (Windows は .venv\Scripts\activate)
python -m venv .venv
source .venv/bin/activate

# 比べる相手と RSS の見張り
pip install highspy==1.15.1 pyscipopt psutil

# 当方: (a) PyPI の wheel (Windows・macOS (Intel・Apple)・Linux、Rust は要らない) か、
pip install enomoto-solver==0.1.10
# (b) 手元のソースからビルド (今のブランチの中身を測るとき。Rust (https://rustup.rs) と maturin が要る)
pip install maturin && pip install -e .
```

- 当方の MPS の読み込みにも highspy を使うので、highspy は (a)・(b) どちらでも要る。
- 行には当方・highspy・pyscipopt の版 (`versions`)、`platform`、CPU の数 (`nproc`) を残す。集計の始めに版の内訳を出すので、途中で版を変えたら分かる。
- 測る間は他の重い作業を止める。ノートパソコンは電源につなぎ、省電力の設定を切る。

## 4. コマンド例

```
# 全部 (53 問 × 5 設定 × 種 1、600 秒)。止めても同じコマンドで続きから回る
python analysis/holdout_20261123/run.py

# 上限・種・設定・問題を選ぶ
python analysis/holdout_20261123/run.py --tl 600 --seeds 1 --cfgs ours_t1,scip
python analysis/holdout_20261123/run.py --tl 60 --seeds 1,2 --problems bab2,ex9,neos-848589
python analysis/holdout_20261123/run.py --problems @mylist.txt --out res_part.jsonl   # 1 行 1 問のファイル

# 集計 (既定は res.jsonl。上限の違う行が混ざっているときは --tl で選ぶ)
python analysis/holdout_20261123/summ.py
python analysis/holdout_20261123/summ.py res.jsonl res_part.jsonl --tl 600 > summ.txt
```

- 再開: `--out` のファイルに同じ (問題, 設定, 種, 上限) の行があれば飛ばす。取得の失敗 (`fetch_error`) と `crash` の行は次の回にやり直す。
- 集計 (summ.py): 状態の内訳、解けた数 (全設定がそろった組で)、最大 RSS、食い違い (設定どうしの最適値の不一致・最適と実行不能の衝突、
  MIPLIB の既知の値との不一致)、ours_t1 / scip・ours_t1 / highs_t1・ours_t4 / highs_t4 の SGM の比 (shift 1 秒、解けない回は上限で数える、
  問題単位のブートストラップの 95% 区間。1 より小さければ当方が速い)、片方だけ解けた数、2 倍以上負けた・勝った問題の一覧。

## 5. 所要時間の見積り (上限 600 秒・種 1)

この 53 問の 600 秒の結果はまだないので、近い性質の Mittelmann 186 問 (600 秒) の 1 回あたりの平均の実時間 (待ち・読み込みを含む) で見積もる
(analysis/vsscip_20261117 の def_t1・scip、vsscip・vshighs_20261117 の def・highs (4 スレッド)。highs_t1 は計測がないので 450 秒と置く)。

| 設定 | 1 回の平均 (秒) | 53 問 (時間) |
|---|---|---|
| ours_t1 | 424 | 6.2 |
| ours_t4 | 355 | 5.2 |
| scip | 413 | 6.1 |
| highs_t1 | (450) | 6.6 |
| highs_t4 | 390 | 5.7 |
| 計 | | **約 30 時間** |

- 上限は 53 × 5 × (600 + 待ち 30) 秒 ≒ **46 時間** (全部が時間切れのとき)。benchmark 集合は難しい問題が多いので、30〜46 時間の間を見ておく。
  設定ごとに分けて回せば (`--cfgs`) 1 設定 5〜7 時間で、夜ごとに 1 設定ずつ回せる。
- 4 スレッドの設定は CPU を 4 つ使う。CPU が 4 未満の機械では ours_t4・highs_t4 の結果は当てにならない。
- メモリ: square47・k1mushroom・s100 などは大きい。手元の機械のメモリが 16 GB 未満なら、まず小さい問題から (`--problems`) 回して最大 RSS を見る。

## 6. 台本の確かめ (この環境、結果はコミットしない)

ex9・neos-2987310-joes・triptim1 を上限 10 秒で、5 設定 × 種 1 を 1 回ずつ流した (`--out` は作業用の場所)。
- 15 回とも結果の行が書かれた (当方は 1 回目、ビルドの作り直し忘れで crash → 作り直して同じコマンドで回すと、crash の 6 回だけやり直し、
  済んだ 9 回は飛ばした。もう一度回すと残り 0 回)。
- summ.py: 食い違い 0。neos-2987310-joes の最適値は ours_t4・highs_t1・highs_t4 で一致 (−607702988.29)。比べ・負けた問題の一覧も出る。
- 当方の ours_t4 は ex9 で上限 10 秒に対し 15.8 秒で返った (時間は求解の時間。上限が短いと超えが目立つ。600 秒では小さい)。
