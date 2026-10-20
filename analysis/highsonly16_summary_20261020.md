# HiGHS だけが解ける 16 問: 9 つの調査のまとめ (2026-10-20)

- 環境: クラウドのコンテナ (CPU 4)。調査はそれぞれ専用の 4 コアのセッションで、両ソルバーとも 4 スレッド・60 秒 (一部 120〜300 秒)。
- 基準: develop bc89ad1。HiGHS 1.15.1。基準の計測は `benchmarks/*_4t_60s_20261020_bc89ad1.*`。
- 各調査のメモ: 下の表の「メモ」の列。計測ログは各 `inv/<担当>-20261020` ブランチに残し、develop には入れていない。
- 試作はどれも既定では使わない (環境変数で有効)。

## 1. 問題ごとの主因

| 問題 | 主因 | 側 | メモ |
|---|---|---|---|
| nursesched-sprint02 | d1bfefd (FJ2 を既定に) の後退。FJ2 の質の悪い解が暫定解になり、根の Pump・ダイビングが回らず、再スタートが起きない | 主側 | regress_..._20261020.md |
| hypothyroid-k1 | 同上。暫定解なし用のダイビング式 RENS (最適値 -2851 を出していた) が回らない | 主側 | 同上 |
| neos-950242 | 9946555 (対称性の検出の近道) の後退。lex-leader の行に反する解 (4・22) を捨てる | 主側 | 同上 |
| ns1116954 | 4 スレッドでは b38653b も bc89ad1 も解なしで判定できない (根のカットのループの LP が時間切れ) | 不明 | 同上 |
| drayage-25-23・drayage-100-23 | クリークカットを既定で使わない (HiGHS の根の下界の伸びはクリーク + implied bound)。前処理の大きさは主因でない | 双対側 | drayage_20261020.md |
| beasleyC3・mc11 | 木の中の分離がほとんど走っていない (プールのラウンドで `resolves` が進み分離が飛ばされる)。mc11 は木の下界の伸びも弱い | 双対側 | beasleyC3_mc11_20261020.md |
| physiciansched6-2 | 解を出せる手段が FJ だけで、FJ は前処理後の形 (HiGHS の Probing によるクリークの拡張・同値列の代入) に左右される | 前処理・主側 | physiciansched6-2_20261020.md |
| uccase12・neos-2978193-inde | 木が 5〜9 倍・根が長い (原因は未特定)。HiGHS の既定の種は当たりの回 (種 0〜5 で 26〜97 秒) | 双対側 (木) | uccase12_neos-2978193-inde_20261020.md |
| neos-911970 | 根の下界が弱い (47.03 対 52.1、カット選択) + 木のノードで新しいカットを足さない | 双対側 | neos-911970_20261020.md |
| mzzv42z | 根のカット (HiGHS は presolve=off でもカットで -20829)。揺れは根の RINS の出発点次第 | 双対側 + 主側 | mzzv42z_neos-960392_20261020.md |
| neos-960392 | 主側だけ (根の LP = 最適値)。根の RENS が全部固定で実行不能、RINS -226 止まり | 主側 | 同上 |
| roll3000 | 面の頂点のカットが最良スコアになり、質の閾値で今の解を切るカットが全部落ちて根のカットのループが停滞 | 双対側 | roll3000_20261020.md |
| app1-1 | 主側だけ (下界は根で最適値)。最適解の形が 1 つしかなく木の運で見つかる | 主側 | app1-1_20261020.md |

主因が同じ問題のまとまり:
- **主側 (暫定解が出ない・悪い)**: nursesched-sprint02・hypothyroid-k1・neos-950242 (後退)、neos-960392・mzzv42z の揺れ、app1-1、physiciansched6-2。
- **根のカット**: roll3000、neos-911970、mzzv42z、drayage。
- **木の中の分離**: beasleyC3・mc11、neos-911970。
- **木の大きさ (未解明)**: neos-2978193-inde・uccase12。

## 2. 試作と既定にする候補

効果・副作用は各調査セッションでの計測 (4 スレッド・60 秒、多くは各 2 回)。

| 試作 (環境変数) | 担当の問題での効果 | 副作用 (担当が見た範囲) | 判断 |
|---|---|---|---|
| `ENOMOTO_MIP_SYM_LEX_CANON` | neos-950242 解なし → 4 (FJ_POOR_NOINC と併用時) | awhea・comp21-2idx 変わらず | 候補 (軽い・行が付く問題でしか動かない) |
| `ENOMOTO_MIP_FJ_POOR_NOINC` | nursesched 31.1 秒で最適、hypothyroid -3 → -2851 | 根が 10〜40 秒延びうる。net12 が 1 回悪化 | 候補 (40 問で確認) |
| `ENOMOTO_MIP_OBJ_SUPPORT` | app1-1 10.9〜14.0 → 6.3〜7.0 秒 (7 回とも根で) | 40 問で当たるのは graph20-20-1rand だけ (条件で飛ばす) | 候補 |
| `ENOMOTO_MIP_CUTSEL_FACE_ANCHOR=1` | roll3000 根 11.8k → 12.25k、60 秒の下界 12.3k → 12.54k (解けない) | misc07 良、pk1 +5 秒、qnet1 +0.5 秒 | 候補 |
| `ENOMOTO_MIP_MERGE_REPRESOLVE` + `ENOMOTO_MIP_PROBE_EQUIV` | physiciansched6-2 の前処理後が小さく・根の LP が上がるが解なし | 40 問で働く 7 問で悪化なし、timtab1・noswot 良 | 候補 |
| `ENOMOTO_MIP_ROOT_WIDE_SUBMIP` | neos-960392 -226 → -236、mzzv42z -8.6k〜-16.8k → -20190 | binkar10_1 良、rout +4 秒 | 候補 (条件を絞る余地) |
| `ENOMOTO_MIP_FC_NODE_SEP2` | beasleyC3 時間切れ → 19.6〜52.2 秒 (5 回中 4 回) | 判定に当たる 20 問で解けた数 +1、h80x6320d +2〜5 秒 | 候補 |
| `ENOMOTO_MIP_NODE_CUT_ADAPT` | neos-911970 時間切れ → 3 回中 2 回 49〜58 秒 | binkar10_1・10teams 良、rout +3〜6 秒・misc07 +1.5 秒 | 候補 (40 問で判断) |
| `ENOMOTO_MIP_CLIQUE_CUTS=auto` (inv/drayage) | drayage-25-23 18 秒 (1/3 回) | 30n20b8 が解なし | 取り込まない |
| `ENOMOTO_MIP_CLIQUE_EXTEND`・`ENOMOTO_MIP_INEQ_SINGLETON` | physiciansched6-2 の根の LP を上げる | LP が重くなる恐れ・40 問に当たらない | 取り込むが候補にしない |
| `ENOMOTO_MIP_DEGEN_SCORE`・`ENOMOTO_MIP_DROP_FUTILE_CUTS` (inv/uccase12) | 効果なし | - | 取り込まない |

候補の計測 (40 問 + 16 問、4 スレッド・60 秒) の結果と採否は §3 に書く。

## 3. 最終の計測と採否

- 3b35332 のビルド (クラウド 4 コア、両ソルバー 4 スレッド・60 秒)。コンテナどうしで速さが 1.5〜2 倍違うので、新しい既定 (newdef) と
  相手を **同じコンテナで続けて** 測る対を 10 組 (base は 2 組、P1〜P4 は各 2 組) 取った。
- newdef: クリーク行の拡張・行をまとめた後の前処理のかけ直し・同値な 0-1 列の代入・背景のカットの求解への根の基底の受け渡し
  (どれも既定、`ENOMOTO_MIP_NO_CLIQUE_EXTEND`・`NO_MERGE_REPRESOLVE`・`NO_PROBE_EQUIV`・`NO_BG_CUTS_BASIS` で止める)。
  olddef はこの 4 つを止めたもの (= bc89ad1 の既定)。
- P1: `ENOMOTO_MIP_CLIQUE_CUTS=fallback ENOMOTO_T_MIP_CLIQUE_FB_FRAC=0.9`。P2: `ENOMOTO_MIP_FJ_FEAS_EXTEND=1 ENOMOTO_MIP_ROOT_GAP_EXIT=1`。
  P3: `ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1`。P4: P1〜P3 全部。
- HiGHS の基準は `mip_rel_gap=0` で測り直したもの (`benchmarks/*_bc89ad1_highsgap0.*`、40 問 21 問・16 問 15 問を解く)。
  主ギャップの参照値は HiGHS の最適値 (なければ ENOMOTO がどれかの回で最適と示した値)。不一致は相対 1e-4 (計測のスクリプトと同じ)。
- 結果のファイル: `benchmarks/{miplib_results,highsonly16}_4t_60s_20261020_3b35332_<newdef|olddef|p1..p4>_r<組>.*`。

| 組 | 相手 | 40 問 解けた数 new / 相手 | 40 問 SGM (秒) new / 相手 | 不一致 (40 + 16 問) | 16 問 解けた数 new / 相手 | 16 問 主ギャップの和 new / 相手 | 16 問で相手だけが解いた (+) / new だけが解いた (−) |
|---|---|---|---|---|---|---|---|
| basea | olddef | 25 / 25 | 21.74 / 21.72 | 0 | 3 / 2 | 4.03 / 5.06 | −nursesched-sprint02 |
| baseb | olddef | 25 / 25 | 20.16 / 20.51 | 0 | 3 / 3 | 4.00 / 4.39 | - |
| P1a | p1 | 25 / 25 | 22.37 / 20.87 | 0 | 4 / 5 | 3.98 / 4.03 | +drayage-25-23 +uccase12 −neos-960392 |
| P1b | p1 | 25 / 25 | 21.32 / 21.34 | 0 | 2 / 3 | 4.83 / 4.70 | +drayage-25-23 |
| P2a | p2 | 25 / 25 | 20.45 / 21.17 | 0 | 4 / 5 | 3.03 / 2.96 | +ns1116954 |
| P2b | p2 | 25 / 25 | 21.22 / 20.23 | 0 | 5 / 5 | 3.95 / 2.97 | +ns1116954 −uccase12 |
| P3a | p3 | 計測中 | | | | | |
| P3b | p3 | 25 / 25 | 20.29 / 20.51 | 0 | 4 / 4 | 3.97 / 3.00 | +neos-960392 −beasleyC3 |
| P4a | p4 | 計測中 | | | | | |
| P4b | p4 | 25 / 25 | 20.54 / 19.93 | 0 | 4 / 6 | 3.99 / 2.01 | +ns1116954 +drayage-25-23 +neos-2978193-inde −uccase12 |

採否:
- **newdef を既定にする (採用)**。40 問は 2 組とも解けた数 25 / 25・不一致 0・SGM 21.74 / 21.72、20.16 / 20.51 (悪化なし)。
  16 問は解けた数 3 / 2、3 / 3、主ギャップの和 4.03 / 5.06、4.00 / 4.39 と 2 組とも良くなる (neos-950242 が 2 組とも解なし → 19、
  mzzv42z・roll3000 の暫定解も良くなる)。40 問で 2 組とも遅くなったのは pk1 (22.7 / 20.4 → 30.2 / 28.9 秒)、速くなったのは
  binkar10_1 (29.4 / 24.0 → 18.6 / 12.3 秒)。newdef を測った 10 回では 40 問 23〜25 問・不一致 0、16 問 2〜5 問 (平均 3.5)。
- P1〜P4 は既定にしない (候補)。P1 は drayage-25-23、P2 は ns1116954 を 2 組とも解く。どちらも 40 問を落とさない。
  P3 は physiciansched6-2 に解が出る (49524、最適値 49324) が解けない。P4 は 16 問の解ける数が最も増えるが、
  uccase12 の暫定解が 798739 (最適値 11507) まで悪くなる回がある。uccase12・beasleyC3・neos-960392 は同じ設定でも
  解あり / なしが入れ替わる問題なので、1 組だけの +/− は揺れの範囲。
- 計測の後に見つけて直したもの: d492cc5 の newdef の 1 回で beavma が誤った最適 (383738.6) を返した。整数の LP 解を暫定解として
  捨てたときにノードも捨てていたのが原因で、224c62a で直した (`analysis/beavma_wrong_optimal_20261020.md`)。上の計測には入っていない。
