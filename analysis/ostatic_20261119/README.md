# 静的な orbitopal fixing の固定に衝突解析の理由を付ける試作 (2026-11-19、既定は変えない)

依頼 (親、利用者の発案): オービトープを行の順を最初に決める静的な orbitopal fixing で扱い、その固定に衝突解析で使える理由を付ける。
静的な形は「列が辞書式で広義減少」という大域的に正しい制約なので固定は含意で、orbital fixing (支配の刈り、analysis/ofreason_20261119)
のような食い違いにならない (SCIP のオービトープ制約と同じ立場)。

## 試作 (src/mip/solver.rs、どれも既定では使わない)

- `ENOMOTO_MIP_ORBITOPE_STATIC=r` (値に `r`) か `ENOMOTO_MIP_ORBITOPE_REASON=1` ([`orbitope_reason`]): 静的なオービトープの 0-1 の固定に理由の行を付ける。
  - 静的なオービトープ = `ORBITOPE_STATIC` なら全部、既定 (動的) なら `ORBPP` が静的と決めたもの (`static_pp`、根の固定と同じ行の順)。
    動的なオービトープ (道ごとの順) の固定は従来どおり外部の固定 (理由なし)。
  - 理由の行 (`orbitope_reason_row`): 固定 x_j = v を導いた成分の組 L (固定されている 0・1、大域的に固定されたものは除く) について
    v = 0: x_j + Σ_{L の 1} x_k − Σ_{L の 0} x_k ≤ |L の 1|、v = 1: −x_j + (同じ) ≤ |L の 1| − 1。組が成り立てば x_j = v を強い、成り立たなければ冗長
    なので、オービトープの制約を満たす全ての解で成り立つ。カットの写しの表 (`cut_arena`) に入れ、境界の変更にその番号を付ける (`set_cut_tag`)。
    衝突解析 (HiGHS 式、既定の経路) はその行で説明して遡る (`CONFLICT_PROOF_EXPLAIN` の仕組みを共有)。
  - 小さい理由 (`orbitope_reason_small`): 列が辞書式で広義減少なら、行の前の部分 (0..=i) と連続する列の範囲だけの部分行列でも列は辞書式で
    広義減少する。固定 x_{i,c} = v が (行 0..=i, 列 c−1・c)、(行 0..=i, 列 c・c+1)、(行 0..=i, 列 0..=c)、(行 0..=i, 列 c..)、(行 0..=i, 全ての列)、
    次に全ての行で同じ順、の部分のオービトープの固定で導ければ、その部分の固定だけを組にする。どれでも導けなければ全体の組 (400 を超えれば理由なし)。
    表の非零の上限 `ENOMOTO_T_MIP_ORBITOPE_REASON_NNZ` (2e7)。
  - 正しさの単体テスト `orbitope_sub_reason_brute_force` (src/mip/symmetry.rs): 3 行 × 4 列までの乱数の固定 600 組で、全ての部分 (行の前の部分 ×
    連続する列の範囲) の固定が「全体が辞書式で広義減少 (パッキングなら行の和 ≤ 1)」で部分の中の固定だけを満たす全ての行列で成り立つことを総当たりで確かめる。
- 同じ成分で混ぜない: orbital fixing はオービトープの列を動かす生成元を外して使う (既存、src/mip/mod.rs)。`ORBITOPE_STATIC` では動的な固定・ORBPP を使わず、
  根の固定 (x_{i,c} = 0, c > i) は木の固定と同じ行の順 (パッキングの行に置き換えた `o.vars`)。LexRed は既定で使わない。
- ヒューリスティクスの解: 対称性の行・固定に反する解はオービトープの列を辞書式に並べ替える (既存の `Orbitope::canonicalize`)。それでも反する解も、
  元の問題で実行可能なら目的値を暫定値に使う (`ENOMOTO_MIP_SYM_SST=x` と同じ `orig_only`)。
- 再スタート: 新しい求解はリテラルの衝突のプールを持ち越さない。理由を通った衝突で大域的に固定された列は持ち越すが、これは静的な形の根の固定と同じ扱い。

## 確かめ

- `cargo test --release --lib`: 411 通過・0 失敗 (上の単体テストを含む)。
- fuzz (`fuzz.sh`、ログ `fuzz/`): ostr = `ORBITOPE_STATIC=r`、orr = `ORBITOPE_REASON=1` + `ENOMOTO_T_MIP_ORBPP_MIN_COLS=0`・`_MIN_ROWS=1` (小さいオービトープでも
  ORBPP を働かせる)。

| fuzz | ostr | orr |
|---|---|---|
| `mip_fuzz_symmetric.py 300 0 1` | bad 0 | bad 0 |
| `mip_fuzz_symmetric.py 300 0 2` | bad 0 | bad 0 |
| `mip_fuzz_symmetric.py 150 1000 3` (ログつき) | bad 0 (理由の行が付いた問題 8) | bad 0 (7) |
| `mip_fuzz_symmetric.py 150 2000/2150 5` (ログつき) | bad 0・bad 0 (理由の行 1.3 万・1.9 万本、22 問) | bad 0・bad 0 (1.5 万本、6 問) |
| `mip_fuzz_vs_highs.py 300` | bad 0 | bad 0 |
| `fuzz_of.py 1000 0` | bad 0 | bad 0 |

  規模 5 の ostr (種 2150〜) では衝突解析が理由の行で説明した回数が計 12.9 万回 (`CUT_REASON_EXPLAINED`)。網羅は薄い (多くは根で解ける)。

## 手元の比べ (graph20-20-1rand、`ab.jsonl`、ログ `logs/`)

1 スレッド (種なし、120 秒): def 15.2 秒・4294 ノード、st (`ORBITOPE_STATIC=1`) 35.3・3940、ostr 37.2・3681 (衝突の平均の長さ 33.7、st は 12.5)、
**orr 13.6・3762** (理由の行 23286 本、衝突解析で 5540 回説明)。最初の版 (全体の組を理由にする) は ostr で 42.2 秒・5507 ノード (平均の長さ 64.7) と悪かった。

4 スレッド・120 秒 (秒・ノード):

| 種 | def | orr | ostr | st |
|---|---|---|---|---|
| 1 | 6.56・5132 | 6.05・3935 | 45.53・12969 | 32.9・9163 |
| 2 | 5.25・3743 | 5.67・3392 | 40.63・13137 | 38.78・12093 |
| 3 | 5.41・3349 | 5.26・2859 | 41.36・11887 | 29.75・9435 |

- orr はノードを 9〜23% 減らすが時間はほぼ同じ。ostr・st (全部静的) は ORBPP_REPROBE が働かないので graph20 では 6〜8 倍遅く、ostr は st より遅い。
- 今の ORBPP の静的な形 (列 10 以上のパッキング) が働くのは P・M の中で graph20 だけなので、orr が効くのは実質 graph20 だけ。ostr は全てのオービトープに効く。

## 計測の分担表 (まだ回さない。割り振りは親が SST=x の結果を見てから)

- 対象: オービトープのある問題 (analysis/vsscip_slow_20261118/g1/scan.txt)。P 11 問 (graph20-20-1rand・misc07・neos-1456979・neos-827175・neos-911970・
  neos-960392・neos8・noswot・physiciansched6-2・rout・sp150x300d)、M 10 問 (fast0507・neos-1456979・neos-2978193-inde・neos-3381206-awhea・neos-873061・
  neos-933966・neos-960392・sct2・uccase12・graph20-20-1rand)。× 種 1〜3 × {def, orr, ostr}、4 スレッド。
- `shards_P.json` (60 秒、2 分担、各 0.8〜0.85 時間): `cd analysis/ostatic_20261119 && python run.py P K` (K = 0, 1)
- `shards_M.json` (600 秒、14 分担、各 1.0〜1.5 時間): `python run.py M K` (K = 0〜13)
- 集計: `python summ.py P orr`・`python summ.py P ostr` (M も同じ)。結果は `res_SET_sK.jsonl`、途中から再開できる。

## 結論 (利用者の判断、2026-11-19): 計測は打ち切り

- 静的なオービトープ (ostr・orr) の P・M の計測は回さない。上の分担表 (`shards_P.json`・`shards_M.json`) は使わない。
- 理由: (1) 同じく動的な対称性の扱いを静的な行・固定に置き換える SST=x の P・M の計測 (analysis/sstx_20261119) で sst / def が M +16.4%・P +6.5% と悪く、
  解の写しの効きも見えなかった。遅さは動的な扱いを静的なものに置き換えた側から来ている。(2) graph20 の手元の数字で、全部を静的にする ostr は
  41〜46 秒 (既定 5〜7 秒) と大きく遅く、ORBPP の静的なものだけに理由を付ける orr はノードを 9〜23% 減らすが時間はほぼ同じで、
  orr が働くのも P・M では graph20 だけ。
- 試作 (`ENOMOTO_MIP_ORBITOPE_STATIC=r`・`ENOMOTO_MIP_ORBITOPE_REASON`) は既定オフのまま残す (理由の行の正しさは単体テストと fuzz 12 本 bad 0 で確かめてある)。
