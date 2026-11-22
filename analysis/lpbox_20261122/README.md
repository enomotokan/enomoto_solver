# LPBOX: LP 解のまわりに狭めて伝播した問題で FJ・Local-MIP を回す試作 (2026-11-22、利用者の案)

利用者の案: 「LP 緩和の解から最適解があまり離れていない (例: 整数列の距離 3 以下) と仮定し、伝播で縮めた問題を FJ・Local-MIP で解く」。
主ヒューリスティクスとしての試作。仮定は保証されないので、箱は解探しにだけ使い、下界・枝刈りには使わない
(見つかった解は元の問題で実行可能なので、暫定解に試すだけ)。

## 結論 (先に)

- 正しさ: fuzz 7 本と cargo test を LPBOX 付きで回し、誤答 0 (2 節)。fuzz_objint の 200 問では 84 問で LPBOX が回り、22 回暫定解を出した。
- 少数問題 (12 問 × 種 1〜3 × 4 スレッド、6 問 × 1 スレッド、60 秒、3 節) では良さは見えない。
  - 解けた数: def 25、lpbox 24 / 42。mzzv42z 種 1 は def が 59.8 秒で解き、lpbox は時間切れ。
  - SGM (ずらし 1 秒): 比 1.040 (lpbox が遅い)。最初の解の時間の SGM は def 3.56 秒、lpbox 3.59 秒でほぼ同じ。
  - 良く出た問題: neos-1456979 種 2 の時間切れの暫定解が 349 → 180。roll3000 種 2 の最初の解が 3.66 → 1.38 秒。
  - 悪く出た問題: blp-ar98 は 3 種とも LPBOX の解 (6567〜6633) が最終の暫定解になり、def (6400〜6510) より悪い。
    LPBOX が根のヒューリスティクスの前に暫定解を置いたことで、後の RENS・RINS の道筋が変わったと見られる。
  - LPBOX が何も出さない問題: mzzv42z・30n20b8・neos-1456979・physiciansched6-2 (4 スレッド)。
    hypothyroid-k1・nw04 は根のカットのループの後に分数の列がないか、ループの中で閉じて LPBOX まで来ない。

## 1. 作り (src/mip/lpbox.rs、`ENOMOTO_MIP_LPBOX`、既定では使わない)

- 置き場所: 根のカットのループの後、T1MIX と同じ所 (FJ のスレッドを止める前) で、主のスレッドが 1 回回す。
  - 根の FJ のスレッドは根の LP より前に始まり、x̄ を持たないため。親の案 (FJ のスレッドの予算を分ける) からの変更。
- 箱: 根のカットのループの後の LP 解 x̄ で作る。
  - 一般整数列: [⌊x̄⌋−k+1, ⌈x̄⌉+k−1] ∩ 元の境界 (k = `ENOMOTO_T_MIP_LPBOX_K`、既定 3)。
  - 0-1 列: LP で整数値のものを固定 (RENS の既定と同じく全部)。
  - 連続列: 元のまま。
- 伝播: 今の定義域の伝播で締める。実行不能なら k を倍にし、0-1 の固定を半分 (1 つおき) → なしにしてやり直す
  (`ENOMOTO_T_MIP_LPBOX_TRIES`、既定 3 回)。SUBFIX の目標固定率は使っていない。
- 解探し: FJ2 を x̄ を丸めた点から、その後 Local-MIP を x̄ を丸めた点から回す (FJ の解があれば受け取って磨く)。
  見つけた解は良い順に暫定解に試す (ラベル `LPBOX FJ`・`LPBOX Local-MIP`。3 節の計測の時点ではどちらも `LPBOX`)。
- 予算: FJ のスレッドの Local-MIP の上限 (`ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC` 0.1 × 時間上限、10 秒まで) の
  `ENOMOTO_T_MIP_LPBOX_SHARE` (0.5) 倍。根までの秒と残り時間で頭打ちにする。
  - FJ に `ENOMOTO_T_MIP_LPBOX_FJ_FRAC` (0.3) を回す。Local-MIP は `ENOMOTO_T_MIP_LPBOX_STALL` (0.5) × 予算の間改善がなければ止める。
  - 時計は今の FJ・Local-MIP と同じ壁時計。
- 文字:
  - `n`: 0-1 列を固定しない。
  - `s`: FJ のスレッドの上限を半分にする (予算を分ける形。FJ のスレッドはふつう根のカットのループの終わりで止められるので、効くのは長い根だけ)。
  - `l`: FJ を回さない。
  - `t`: 予算を手間で数える (決定的)。解探しの間だけ主のスレッドの時計を手間 ([`crate::ticks::set_vclock`]、
    `ENOMOTO_MIP_TICKS` の `j` と同じ物差し) にし、根までの秒の代わりにここまでの主の手間で頭打ちにする。残り時間の頭打ちはしない。
  - `d`: 箱の LP から根の丸めとダイブを回す (4 節)。
- ログ: `MIP: LPBOX: k=…, fixed … of … LP-integral binaries, boxed … of … general integers, propagated …, found …, obj …, accepted …`。
- 0-1 主の問題向けの別形 (反転本数 ≤ k の行を FJ の問題に足す) は作っていない。

## 2. 正しさ

`bash analysis/lpbox_20261122/fuzz.sh` (analysis/candmem_20261121/fuzz.sh と同じ 7 本と cargo test を `ENOMOTO_MIP_LPBOX=1` で。ビルド 6b941489e)。

| 命令 | 結果 |
|---|---|
| fuzz_tiny 400 0 / 400 1000 | bad 0 / bad 0 |
| mip_fuzz_vs_highs 300 | bad 0 |
| fuzz_objint 1000 0 / 1000 5000 | bad 0 / bad 0 |
| fuzz_slack 1000 0 / 1000 5000 | mismatches 0 / 0 |
| cargo test --release --lib | 419 passed |

fuzz の台本はログを捨てるので、LPBOX が回っているかを別に見た: fuzz_objint の種 0〜199 で 84 問に LPBOX の行があり、22 回暫定解になった。

## 3. 少数問題 (def と lpbox、60 秒)

- ビルド: c6ab6b2b4 (pip install -e .、.so の sha1 f6707aefb8b72317bed1a3408ced562d2f27a054)。
- 台本: `run_small.py plan_s4.json` (4 スレッド、種 1〜3) と `run_small.py plan_s1.json` (1 スレッド、種 1)。
  結果は `res_small.jsonl`、ログは `logs/`、集計は `python summ_small.py > summ_small.txt`。
- 問題 (MIPLIB の 58 問から。analysis/final_20261118 の def の最初の解の時間と、MPS の一般整数列の数で選んだ):
  - 最初の暫定解が遅い問題: mzzv42z (14〜18 秒)、nw04 (16 秒)、neos-1456979 (2〜13 秒)、physiciansched6-2 (9〜12 秒)、
    hypothyroid-k1 (9〜11 秒)、30n20b8 (8〜10 秒)、neos-860300 (6〜7 秒)、blp-ar98 (5 秒)。
    mzzv42z は一般整数列も 235 本ある。
  - 一般整数の多い問題: roll3000 (492 本)、timtab1 (94 本)、gen-ip054 (全部)、mik-250-20-75-4 (175 本)。
  - 1 スレッド: roll3000・timtab1・neos-1456979・hypothyroid-k1・30n20b8・mik-250-20-75-4。

結果 (summ_small.txt から):

| | def | lpbox |
|---|---|---|
| 解けた数 (42 回) | 25 | 24 |
| SGM (ずらし 1 秒、時間切れは 60 秒) | 22.82 | 23.76 (比 1.040) |
| 最初の解の時間の SGM | 3.56 | 3.59 |

LPBOX が暫定解を出した (受け入れた) 回: blp-ar98 3 種、gen-ip054 3 種、roll3000 3 回 (うち 1 スレッド 1 回)、timtab1 3 回、30n20b8 の 1 スレッド。
ただしその後に RENS・RINS・木が良い解を出すので、最終の暫定解になったのは blp-ar98 だけ (しかも def より悪い)。

時間切れの暫定解 (def → lpbox):

| 問題 | 種・スレッド | def | lpbox |
|---|---|---|---|
| blp-ar98 | 1・4 | 6503.2 | 6567.1 |
| blp-ar98 | 2・4 | 6400.3 | 6633.1 |
| blp-ar98 | 3・4 | 6510.5 | 6610.1 |
| neos-1456979 | 1・4 | 191 | 195 |
| neos-1456979 | 2・4 | 349 | 180 |
| gen-ip054 | 1・4 | 6841.0 | 6848.3 |
| 30n20b8 | 1・1 | なし | 453 |

- 一般整数列の箱はほとんど効いていない: roll3000 で一般整数 114 列 (前処理の後) の LP 値はほぼ整数で、k = 3 の箱が元の境界より広く、締まる列が 0。
  効いているのは 0-1 の固定 (RENS と同じ近傍) と伝播。
- mik-250-20-75-4・neos-860300 では LPBOX の解 (−51104〜−51612、5217) が既にある暫定解より悪く、受け入れられない。
