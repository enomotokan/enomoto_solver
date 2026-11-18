# gate2 候補 8: CUTCONF の長さのゲート (cclg) と INOUT_SEP (2026-11-18)

BRIEF (../BRIEF.md) の候補 8。試作の環境変数 `ENOMOTO_MIP_CUTCONF_LGATE` (既定では使わない)。比べる相手は今の既定 (a9d7bae3d の上)。

## 1. 分ける特徴 (記録 analysis/cutconf_20261110 の p58・p58w、58 問 60 秒、種 1・2)

- CUTCONF (形 cc・ccb・ccw) で 2 種とも速いのは noswot (log 比 −1.0 前後) と qiu (−0.5) だけ。physiciansched6-2・exp-1-500-5-5 は
  作った制約が 0〜27 本で締めも 0 なので揺れ。遅いのは 10teams・neos5・h80x6320d・drayage-25-23 (ccw)・rout・eil33-2・pk1。
- 一番よく分けるのは、作った制約の平均の長さ L (`MIP: cutconf` の行の mean length)。速い 2 問は 13〜17、遅い問題は
  10teams 250・eil33-2 1150・h80x6320d 220〜300・drayage 180〜300・rout 70・pk1 53・neos5 33。一般整数の割合 (noswot 1.0、qiu 0.03)・
  刈りと締めの比では分かれない。境の近くに binkar10_1 (L≈25、+0.1)・neos5 (33) がある。しきい値はこのデータを見て選んだので楽観的。

## 2. 試作 `ENOMOTO_MIP_CUTCONF_LGATE=L0,s,K` (`ENOMOTO_MIP_CUTCONF` と一緒に使う)

- `src/mip/cutconf.rs` の `lgate_weight`・`lgate_take`、`src/mip/solver.rs` の `add_cutconf`。
- 全スレッドで今までに作った制約の平均の長さ L から、入れる割合 w = σ((L0 − L)/s) を決める (作った数が K 本に満たない間は w = 1)。
  スレッドごとに w を足していき 1 を超えたら入れる (乱数を使わない)。w < 1e-3 なら解析もしない (以後止まる)。既定は 25,5,20。
- 制約を捨てるだけなので正しさは変わらない。

## 3. 確かめ

- 誤答 (`fuzz.sh`、試作の環境変数つき。`LGATE=5,2,5` で捨てる側の道も通す): fuzz_tiny 400×2・mip_fuzz_vs_highs 300・fuzz_objint 1000・
  fuzz_slack 1000・fuzz_cutconf 300×2 で bad 0。cargo test (既定) 419 通過・0 失敗。
- 手元 (60 秒、種 3、1 回ずつ、`one.sh`、ログは logs/)。秒、括弧はノード:

| 問題 | def | cc | cclg | cc の L |
|---|---|---|---|---|
| noswot | 56.7 | 40.3 | 9.1 | 15.8 |
| qiu | 10.1 | 10.2 | 10.2 | 19.6 |
| 10teams | 3.4 (0) | 3.9 (0) | 4.2 (0) | 150 (2 本) |
| neos5 | 18.2 | 18.3 | 20.1 | 33.1 |
| h80x6320d | 25.2 | 23.4 | 31.3 | 444 |
| drayage-25-23 | 0.83 | 0.76 | 0.84 | 26.7 |
| rout | 5.7 | 4.2 | 5.0 | 77.5 |
| pk1 | 13.9 | 14.5 | 15.0 | 49.2 |
| eil33-2 | 19.6 | 20.1 | 20.6 | 1163 |
| mas76 | 9.2 | 9.0 | 9.3 | 58.9 |

- 事実: 今の既定 (11-10 の記録より後) では、10teams は根で解け (3.4 秒、記録では 11.6〜12.9)、qiu は 10 秒 (記録では 44〜52)。
  CUTCONF だけ (cc) でも、記録で遅かった問題 (10teams・neos5・h80x6320d・drayage-25-23・rout・pk1・eil33-2) に 1 回では害が見えない。
  noswot は揺れが大きい (記録で既定 18〜104 秒) ので 1 回の差は読めない。
- 推測: 11-10 の害の多くは今の既定で消えていて、ゲートが守るものは小さい。cc と cclg の両方を既定と比べる。

## 4. 広い計測 (しきい値を選ぶのに使っていない種)

- `python plan.py 12 3` で作った分担表: M = Mittelmann 48 問 × 種 3・4 × {def, cclg}、600 秒、12 分担 (1 分担 1.4 時間)。
  P = MIPLIB の 58 問 × 種 4〜6 × {def, cc, cclg}、60 秒、3 分担 (1 分担 1.5 時間)。
- 回し方 (リポジトリの根から): `cd analysis/gate2_20261117/cclg && python run.py P K` (K = 0〜2)、`python run.py M K` (K = 0〜11)。
- 集計: `python summ.py P cclg`・`python summ.py P cc`・`python summ.py M cclg`。

## 5. INOUT_SEP

試作のコード (ブランチ inv/inout-sep-20261022) は origin にもこのブランチにも残っていない。効くのは最適面が広い問題 (neos-911970・
physiciansched6-2 の根の下界) だけで、60 秒で良くなったのは beasleyC3・noswot だけ。特徴 (最適面の広さ) を測るのに内点法が 1 回要り、
記録では GATE=2 でも 10teams・neos-960392・neos-911970 で止まらない。見込みが薄いので、今回は作り直さず区切る。
