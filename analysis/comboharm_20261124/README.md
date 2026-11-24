# combo の害の切り分け (2026-11-24、親の依頼)

MIPLIB 240 問の組み合わせの計測 (analysis/combo_20261124) で、combo (既定候補 8 つを全部入れた形、`cfg.py` の `COMBO`) で遅くなった問題について、
どの部品が原因かを切り分けた。部品の短い名前 (`run.py` の `NAMES`):

| 名前 | 環境変数 |
|---|---|
| sbpb | `ENOMOTO_MIP_SB_PROBEBD=1` |
| cutu | `ENOMOTO_MIP_CUTOFF_USER_TREE=1` |
| bfrt | `ENOMOTO_MIP_NODELP_BFRT_GROUP=1` |
| ppi | `ENOMOTO_MIP_PROOFPROP=i` (2 スレッド以上でだけ働く) |
| heur | `ENOMOTO_MIP_HEUR_ADAPT=1` |
| nra | `ENOMOTO_MIP_NODEROWS_ADAPT=t` |
| enum | `ENOMOTO_MIP_ENUM_ADAPT=1` |
| tie | `ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER=1` |

計測はこの機械 (4 コア) で、1 スレッド・種 1・600 秒。並べて回した本数は 1〜4 本で、時間はその分揺れる。ノード数は、経過時間に依る判定に
当たらない間は決定的 (7 節)。表の「抜く」は combo から 1 つ抜いた形、「足す」は既定に 1 つ足した形。

## 0. まとめ

| 問題 | 原因 | 仕組み | 直し (既定オフ) |
|---|---|---|---|
| neos-631710 | **nra** | 木の中の再スタートの判定 K が、nra の外した行の分だけ「効いているカット」を少なく数えて再スタートを止める | `NODEROWS_ADAPT=tk` (外した行も数える)。+nra 117 秒・combo 202 秒で解ける |
| chromaticindex1024-7 | **enum** | ENUM_ADAPT の 1 周目は行の上限なしに全部列挙する。67567 行の列挙に 449 秒かかり、得は 0 | `ENUM_ADAPT=1s` (1 周目の途中でも同じシグモイドで止める)。根を抜けて木に入る |
| mushroom-best | **sbpb** | 強分枝の子の伝播・両方の子の境界で、木が 2161 → 4631 ノードになる | (未作。sbadapt の f_B で効かせる形を見た: 6 節) |
| chromaticindex512-7 | 再現しない | 既定は時間切れ・499 秒、combo は 342・394・498 秒。この機械では combo のほうが速い | — |
| neos-1456979 | はっきりしない | 既定もこの機械では 600 秒で時間切れ (combo の計測では 393 秒)。combo は根の後のサブ MIP・含意の数から分かれる (heur の見込み) | — |
| gmu-35-40 | はっきりしない | 1 スレッドでは既定・combo とも時間切れ (害が出たのは 4 スレッド)。combo は得た値・下界とも少し悪い | — |

- 確かめ (直し 2 つをつけて、`fuzz.sh fix ENOMOTO_MIP_NODEROWS_ADAPT=tk ENOMOTO_MIP_ENUM_ADAPT=1s`、`fuzz_fix.out`): fuzz 7 本は誤答 0、
  cargo test は 424 通過・0 失敗。直しを入れたビルドでも、既定の道筋は 6 問とも基準と一致した (`check_default.out`)。

## 1. neos-631710 (原因: nra)

`res.jsonl` (1 スレッド、種 1、600 秒、3 本並べて)。

| 形 | 結果 | 秒 | ノード |
|---|---|---|---|
| 既定 | 最適 203 | 142 | 2300 |
| combo | 時間切れ | 600 | 76501 |
| −sbpb・−cutu・−bfrt・−ppi・−heur・−enum・−tie | 時間切れ | 600 | 32499〜89506 |
| **−nra** | 最適 | 144 | 4207 |
| **+nra** | 時間切れ | 600 | 53199 |
| +sbpb・+bfrt | 最適 | 195・199 | 3301・4211 |

### 仕組み (`logs_631710_{def,nra}.err`、`ENOMOTO_MIP_NRA_TRACE=500`・`ENOMOTO_MIP_LOG=1`、200 秒)

- この問題は木の中の再スタートで解ける。既定はノード 2300 (105 秒) で再スタートする。
  - 木の中で暫定解が 220 → 203 に良くなっている。再スタートの後の根では、打ち切り値で 7216 列中 3637 列が固定され、すぐ終わる。
- 再スタートの判定には既定の守り `KR` がかかる (`ENOMOTO_MIP_TREEEFF` の `r` が既定)。src/mip/solver.rs `tree_restart_guard_ok`。
  - K は「新たな固定 < 1% かつ、効いている (等式に近い) カットの行 tcuts < max(50, 0.1 × 問題の行 = 945)」なら止める。
- 既定は tcuts 951 で、ぎりぎり通っていた (`TRSTAT: due ... tcuts 951 ... guard pass`)。
- nra は φ ≈ 0.9 (閉じた葉のほとんどが実行不能) なので f ≈ 1 になる。
  - 取り出したノードで、基底に入っている (効いていない) 木のカットの行をほぼ全部外す。
  - そのため判定の時点の LP のカットの行が少なく、tcuts が 744 → 716 → 696 と下がり、毎回 `guard stop K` で再スタートしない。
- 確かめ (`res_kfrac007.jsonl`・`res_kfrac011.jsonl`):
  - +nra で K のしきい (`ENOMOTO_T_MIP_TREE_RESTART_GUARD_KFRAC`) を 0.07 に下げると 105 秒・2201 ノードで解けた。
  - 既定でしきいを 0.11 に上げても 222 秒で解けた (後の判定で通る)。
- **K の物差しの問題** (記録だけ、別の課題): K は「新たな固定」と「持ち込めるカット」しか見ない。
  打ち切り値の改善 (220 → 203) による再スタート後の根の固定 (この問題の効きの本体) を見ていない。

### 直し `ENOMOTO_MIP_NODEROWS_ADAPT` の `k` (src/mip/solver.rs `node_cleanup_rows`・`nra_k_tight`・`tree_restart_guard_ok`)

- nra が LP から外した行を、指紋 → (係数, 右辺) で取っておく。上限は d2b と同じく LP の行の 2 倍 + 1000 で、超えたら捨てて数え直す。
- 判定 K の tcuts に、外した行のうち今の LP にないもので、今の LP 解で締まっている (または破られた) ものの数を足す。
  - TRSTAT の行には `tcuts N (nra M)` と出す。
- 外す前に締まっていた行を数える案 (親の例) は使わなかった。nra は基底に入っている (締まっていない) 行しか外さないので、外す時点で数えると 0 になる。
  判定の時点の LP 解で数えるのが、nra がなければ LP にあったはずの効いている行の数に当たる。
- 新しい閾値はない。nra をつけないとき・`k` をつけないときは何も変わらない。

| 問題 (1 スレッド、種 1) | 既定 | +nra (`t`) | +nra (`tk`) | combo | combo (nra を `tk`) |
|---|---|---|---|---|---|
| neos-631710 (600 秒) | 142 秒 / 2300 | 時間切れ / 53199 | **117 秒 / 2201** | 時間切れ | **202 秒 / 14593** |
| mik-250-20-75-4 (120 秒) | 16.0 / 6333 | 8.6 / 2407 | 8.6 / 2407 | | |
| roll3000 (120 秒) | 68.0 / 6399 | 68.8 / 6249 | 69.4 / 6249 | | |
| fhnw-binpack4-4 (200 秒、実行不能) | 73.9 / 10977 | 15.1 / 10333 | 15.1 / 10333 | | |

- nra が効いていた 3 問は `t` と `tk` でノード数まで同じで、得はそのまま残る (`res_nra_t.jsonl`・`res_nra_tk.jsonl`)。
- 既定と同じ時刻に再スタートしたか: +nra(`tk`) は 2201 ノードで終わった。既定の再スタート (2300 ノード) と同じころに再スタートして解けたと見られる
  (ログなしの回なので時刻は未確認)。

## 2. chromaticindex1024-7 (原因: enum)

- combo の計測: 1 スレッドで combo は 600 秒でノード 0、既定は 2458 ノードで時間切れ。4 スレッドは既定 444 秒、combo は時間切れ。
- ログ (`logs_ci1024_combo.err`、combo、1 スレッド): `enumeration presolve (after probing): 67567 rows, 319396 solutions, 0 fixed, 0 bounds tightened,
  0 equivalent pairs, 208253 new conflict pairs (449.49s)`。続く `ENUM_ADAPT: loop round 1: gain 0 in 452.46s -> stop`。
- ENUM_ADAPT は中で `ENUM_PRESOLVE=all` を使う。1 周目は、行の上限 (`ENOMOTO_T_MIP_ENUM_ROWS` = 400) と空振りでやめる規則なしに、候補の行を全部列挙する。
  シグモイドの判定 (`enum_adapt_continue`) は周の終わりにしか働かないので、1 周目は止まらない。

### 直し `ENOMOTO_MIP_ENUM_ADAPT` の `s` (`=1s` など、src/mip/mod.rs `enum_adapt_midstop`・`enumerate_rows`)

- 列挙 1 回の途中でも、16 行ごと (前処理の時間の止め `presolve_stop` を見るのと同じ所) に同じシグモイドの判定を当てる。
  - 得 = 固定・締め付け・同値な組 (衝突の組は数えない、`enum_adapt_continue` と同じ)。
  - c = 列挙を始めてからの秒 / 列挙の始めの求解の経過時間。
  - f = sigmoid(a (log10(g / c) − b)) とし、c > max(f, 1/2) なら止める。
- 1 周目には前の周の観測がないので、中立の点 (s = b、f = 1/2) を下限にした。
  - 得がないまま、求解の経過時間の半分を列挙に使ったら止まる。得が良ければ (f > 1/2) それ以上使ってよい。
  - 新しい閾値はなく、a・b は既存の係数。
- **400 行の上限を 1 周目に守る案を採らなかった理由**: ENUM_ADAPT が効く net12 は 953 行全部の列挙 (0.06 秒) で根が 68 → 128 になる (cutgap 5 節)。
  - 上限 400 はその得を削る。列挙はふつう 1 秒前後で終わる (mzzv42z 3817 行 1.4 秒)。
  - 害は「行が多い」ことではなく「得のない列挙に時間を使い続ける」ことなので、時間と得で止めるほうが理屈に合う。

| 問題 (1 スレッド、種 1、60 秒) | +enum (`=1`) 根の下界 / 結果 | +enum (`=1s`) 根の下界 / 結果 |
|---|---|---|
| net12 | 131.74 / 時間切れ (0 ノード) | 131.74 / 時間切れ (0 ノード) |
| istanbul-no-cutoff | 70.44 / 時間切れ | 70.44 / 時間切れ |
| ns1830653 | 11168.1 / 時間切れ | 11168.1 / 時間切れ |
| noswot | −43 / 16.1 秒 | −43 / 16.2 秒 (同じ 20135 ノード) |
| mzzv42z | −20700.6 / 時間切れ | −20700.6 / 時間切れ |
| rout | 990.49 / 18.4 秒 | 990.49 / 17.8 秒 (同じ 5809 ノード) |
| chromaticindex1024-7 (combo、600 秒) | ノード 0 (combo の計測) | 根を抜けて **1379 ノード** (時間切れ。既定も 1 スレッドは時間切れ) |

効いていた 6 問は根の下界がまったく同じで、得は残る (`res_enum_1.jsonl`・`res_enum_s.jsonl`)。

## 3. mushroom-best (原因: sbpb)

| 形 (1 スレッド、種 1、600 秒) | 秒 | ノード |
|---|---|---|
| 既定 | 334 (ログつき)・291 | 2161・2161 |
| combo | 時間切れ (ログつき、4258 ノード)。combo の計測では 408 秒・4303 | |
| **−sbpb** | **347** | **3187** |
| −heur・−enum・−tie・−cutu | 434〜452 | 4303〜4693 |
| −nra・−bfrt | 536・577 | 4985・6131 |
| **+sbpb** | **469** | **4631** |

- 既定に sbpb だけ足すとノードが 2.1 倍になる。ノード数は回をまたいで同じ (2161) で、道筋の揺れではない。
- ログ (`logs_mushroom-best_{def,combo}_t1.err`): 根は同じで、木の最初の数ノードの強分枝 (sb_iters) から分かれる。
- sbpb の害は sbadapt_20261124 でも 30n20b8 で出ていた。sbadapt の f_B (deg で効かせる) の形を mushroom-best で見た結果は 6 節。

## 4. chromaticindex512-7 (再現しない)

| 形 (1 スレッド、種 1、600 秒) | 1 回目 | 2 回目 | 3 回目 (ログつき) |
|---|---|---|---|
| 既定 | 時間切れ (11215 ノード) | 499 秒 (8969) | |
| combo | 342 秒 (2825) | 394 秒 (5017) | 498 秒 (2951) |

- 同じ設定でもノード数が回ごとに変わる (経過時間に依る判定のため、7 節)。
- combo の計測の 198.7 → 257.1 秒 (1 スレッド)・163.5 → 228.0 秒 (4 スレッド) は、この機械では逆向きで、揺れの中と見る。
- chromaticindex1024-7 と同じ列挙の重さは、512-7 では小さいと見られる (ログなしの回は未確認)。

## 5. neos-1456979・gmu-35-40 (はっきりしない)

- **neos-1456979** (`logs_neos-1456979_{def,combo}_t1.err`):
  - この機械では既定も 600 秒で時間切れ (13204 ノード、176 / 171)。combo は 16637 ノード、228 / 168。
  - 根のカットのループは同じ。CUTSEL_PROBE_TIE_FEWER の判定の行が出るが、選んだのは既定と同じ側。
  - その後、サブ MIP の回数と含意の数 (CUTMIX clique edges) が違ってくる。HEUR_ADAPT の見込みだが、既定が解けないので抜いた形の比べはしていない。
- **gmu-35-40**: combo の計測の害は 4 スレッド (373 → 567 秒)。
  - 1 スレッドでは既定・combo とも時間切れ。既定は 766462 ノード、−2406257 / −2406911。combo は 581865 ノード、−2405951 / −2406922。
  - 差が小さく、切り分けていない。

## 6. 参考: mushroom-best の SB_ADAPT=1 (sbpb を f_B の確率で効かせる)

既定に `ENOMOTO_MIP_SB_ADAPT=1` だけ足した形 (`res_sbadapt.jsonl`): **600 秒で時間切れ (6533 ノード)**。

- sbpb をそのまま足した形 (469 秒・4631 ノード) より悪い。
- SB_ADAPT は sbpb (確率 f_B) のほかに、退化のノードでの強分枝・HiGHS 並みの予算 (f_A)・衝突の重みの床 (f_B) も変える。
  mushroom-best ではどれかが害になり、f_B・f_A の物差し (deg・cpn) では止まらなかった。
- mushroom-best の sbpb の害は、sbadapt の形では避けられない。直しの見当はまだない
  (強分枝の子の伝播で何が変わるかを見る必要がある。未調査)。

## 7. 1 スレッドでも結果が回ごとに変わる (経過時間に依る判定)

- neos-631710 の combo + K しきい 0.07 は、3 本並べた回では時間切れ (73301 ノード)、ログつきで 1 本の回では 179 秒で解けた。
- 違いは木の中の再スタートの判定の時刻。ログつきの回はノード 12100 (156 秒) で判定が回り、ノード 13800 で再スタートした。
- 再スタートの判定 (`tree_restart_due`) の呼び出し自体はノード数と刈った重み q で決まる (決定的)。経過時間に依るのは次の 3 つ。
  - **守り `R`** (既定の `KR` の片方): src/mip/solver.rs `tree_restart_guard_ok`。
    - 木の秒 `tree_t` (27973 行、`solve_t − tr_t0`)、根のやり直しの費用 `root_cost` (27995 行、木に入るまでの秒) を使う。
    - q の伸びからの残りの見込み `est_r` (28020 行) を秒で比べる (28055 行の `'R'`)。
    - `ENOMOTO_MIP_TIMEDEP` の `e` をつけると LP の反復で数える (既定ではない)。
  - **残り時間の条件**: src/mip/solver.rs 27936 行 (と 13224 行)。残り時間が `ENOMOTO_T_MIP_TREE_RESTART_MIN_LEFT` (0.1) × 時間上限以上。
  - **木の中の分離の予算** (`tree_sep_budget_ok`、src/mip/solver.rs 7468 行) など、時間で決まる判定が木の道筋を変える。
    - その結果、判定の時点の tcuts・q も変わる (CONFCUT の計測で、候補 0 本の設定でもノード数が 2 倍違った: analysis/confcut_20261124 3 節)。
- 並べて回す本数 (CPU の混み具合) で秒が変わるので、combo の計測 (別の機械・別の並べ方) とこの機械の結果が食い違いうる。
  chromaticindex512-7・neos-1456979 がこれに当たると見る。

## 9. 追加の問題 (親の追加の依頼、1 スレッド、種 1、600 秒。`res_more.jsonl`・`res_more_ks.jsonl`)

combo+k+s は combo の nra を `tk`、enum を `=1s` にした形。この節の回は 4〜6 本を並べて回したので秒の揺れが大きく、ノード数を主に見る。

| 問題 | 既定 | combo | combo+k+s | 見立て |
|---|---|---|---|---|
| rail507 | **最適 588 秒** (4393) | 時間切れ (3217、176 / 173) | 時間切れ (3193、176 / 173) | 害。既定も上限の近く。抜いた形は未測 (1 回 600 秒 × 8) |
| neos-2987310-joes | 23 秒 (0 ノード) | 27・38 秒 | 27 秒 | **enum**。−enum 16 秒、+enum 26 秒、+heur 16 秒。`=1s` のログつき 1 本では 15.5 秒 (下) |
| ns1208400 | 89 秒 (2457) | 137 秒 (8031) | (combo の nra を `tk`: 146 秒・7041) | **組み合わせ**。下 |
| neos-933966 | 171 秒 (388) | 267 秒 (801) | | **組み合わせ / 道筋**。下 |
| tbfp-network | 時間切れ (6 ノード、32.28 / 23.57) | 時間切れ (4、32.28 / 23.61) | 同 combo | 1 スレッドでは差なし (害は 4 スレッドだけ) |
| gen-ip054 | 時間切れ (6840.97 / 6785.26) | 時間切れ (6850.21 / 6782.76) | 同 combo | 1 スレッドでは差が小さい |
| roi2alpha3n4 | 時間切れ (−56.50 / −68.05) | 時間切れ (−56.98 / −69.12) | (−57.47 / −69.12) | 得た値は combo が良く、下界は悪い。差は小さい |
| peg-solitaire-a3 | 時間切れ (解なし、3564 ノード) | 時間切れ (解なし、5722) | | 1 スレッドでは差が分からない |

(括弧の中は ノード、または 得た値 / 下界。)

- **neos-2987310-joes (enum)**: 列挙の 1 周目が、得のない行を列挙し続ける (chromaticindex1024-7 と同じ形の小さい版)。
  - `ENUM_ADAPT=1s` のログ (`logs_joes_enum_s.err`): `stopped the enumeration after 432 of 2800 rows: gain 0 in 2.57s (5.12s elapsed at the start)`。
  - 全体は 15.5 秒で、既定並みに戻った。
- **ns1208400 (組み合わせ)**:
  - 1 つずつ抜いた形のノード数: −nra 1685、−bfrt 3887、−heur 5897、−sbpb 5957、−enum・−tie・−cutu 8031 (= combo)。
  - 既定に nra だけ足すと 1997 ノード・89 秒で、既定 (2457) と同じかよい。
  - 再スタートはどの回でもしない (ログ `logs_ns1208400_{def,nra}.err`)。判定 K の問題ではなく、直し k でも消えない。
  - nra を抜くと速く、nra だけ足しても害はないので、nra とほか (bfrt・heur・sbpb) の組み合わせで木が大きくなっている。
    φ ≈ 0.9 で nra はほぼ全部の行を外す側にいる。外し方と強分枝・LP の道筋が重なった結果と見るが、仕組みは未調査。
- **neos-933966 (組み合わせ / 道筋)**:
  - 1 つずつ抜いた形のノード数: −bfrt 292、−sbpb 401、−heur 568、−enum・−tie・−cutu 801 (= combo)、−nra 1088。
  - 既定に 1 つ足した形は、+sbpb が時間切れ (1916 ノード、得た値 1359)、+bfrt も時間切れ (4621 ノード、321)、+heur も時間切れ (1247 ノード、1338)。
    どれも combo よりかえって悪い。
  - 部品の足し引きでノード数が大きく上下し、単調でない。道筋の運が大きいと見る (並べた本数が多く、経過時間に依る判定も混じる)。
- **rail507**: 抜いた形は 600 秒 × 8 回かかるので、まだ回していない。

直し k・s で消えた害: neos-631710 (k)・chromaticindex1024-7 (s、木に入るようになる)・neos-2987310-joes (s)。
消えない害: mushroom-best (sbpb)・ns1208400 (組み合わせ)・neos-933966 (道筋)・rail507 (未切り分け)。

## 8. 再現

```
python analysis/comboharm_20261124/run.py analysis/comboharm_20261124/res.jsonl neos-631710 1 def,combo,-sbpb,-cutu,-bfrt,-ppi,-heur,-nra,-enum,-tie --par 3
python analysis/comboharm_20261124/run.py analysis/comboharm_20261124/res.jsonl neos-631710 1 +nra,+sbpb,+bfrt --par 3
CH_EXTRA="ENOMOTO_MIP_NODEROWS_ADAPT=tk" python analysis/comboharm_20261124/run.py analysis/comboharm_20261124/res_fixk.jsonl neos-631710 1 +nra,combo
CH_EXTRA="ENOMOTO_MIP_ENUM_ADAPT=1s" python analysis/comboharm_20261124/run.py analysis/comboharm_20261124/res_enum_s.jsonl chromaticindex1024-7 1 combo
bash analysis/comboharm_20261124/logrun.sh neos-631710 combo 1 600     # ログつき (logs_*.err、コミットしない)
bash analysis/comboharm_20261124/fuzz.sh fix ENOMOTO_MIP_NODEROWS_ADAPT=tk ENOMOTO_MIP_ENUM_ADAPT=1s
```

`CH_EXTRA` は設定の後に足す環境変数 (空白区切り)。直しの確かめの回は、走っている計測の .so を置き換えないよう別の venv に入れたビルドで回した
(ソースは同じコミット)。
