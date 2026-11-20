# OBJTARGET: FJ・Local-MIP の「目的値 ≤ T」の目標と、並列の仮の暫定値つき前処理 (2026-11-19〜20、親の依頼)

既定の動作は変えない (試作は環境変数の下)。

## 1. 対象の数え上げ (事実)

- 最終計測 (analysis/final_20261118、def、4 スレッド、種 1) の M 186 問のうち参照値 (scripts/miplib2017-v31.solu) のある 181 問で、
  600 秒の終わりに主のギャップ > 1%・解なし・最初の解が 60 秒より後のいずれかの問題 54 問を、今の版 (既定) で 60 秒 (600 秒の 1 割)・ログつきで
  回し、60 秒の時点の暫定解の主のギャップを数えた (`traj.py`、`res_traj_M60.jsonl`。同時に cargo test を回していた時間があり、時刻は少し遅め)。
- **60 秒で主のギャップ > 5% か解なし: 45 問** (解なし 13: academictimetablesmall・satellites2-60-fs・satellites2-40・rail01・
  neos-3216931-puriri・dws008-01・cmflsp50-24-8-8・ns1952667・neos-1354092・bnatt400・neos-2746589-doon・germanrr・peg-solitaire-a3。
  解はあるが > 5%: brazil3 0.92・markshare2 0.94・trento1 0.85・neos-5107597-kakapo 0.85・neos-5188808-nattai 0.85・momentum1 0.79・mad 0.75・
  comp21-2idx 0.68・neos-5195221-niemur 0.69・sing326 0.98・neos-933966 0.99・neos-3024952-loue 0.45・rococoB10-011000 0.44・lectsched-5-obj 0.42・
  rahue 0.31・rocII-5-11 0.30・glass4 0.29・toguru 0.27・opm2-z10-s4 0.23・csched007 0.14・traininstance2 0.13・sorrell3 0.13・cvs16r128-89 0.10・
  leo2 0.10・neos-873061 0.10・nexp-150-20-8-5 0.09・tavua 0.08・blp-ic98 0.07・kumeu 0.06・b1c1s1 0.06・uct-subprob 0.05・sp97ar 0.05)。
  ≤ 5% は 6 問 (atlanta-ip・supportcase18・radiationm40-10-02・leo1・thor50dday・supportcase42)。
- 下界側が弱い問題 (600 秒で時間切れ、終わりの主のギャップ ≤ 0.1%): M 24 問 (30n20b8・assign1-5-8・bppc4-08・chromaticindex・gen-ip002・
  gen-ip054・glass-sc・gmu-35-40/50・graphdraw-domain・milo-v12-6-r2-40-1・crna・murg・neos-662469・net12・ns1208400・ran14x18-disj-8・
  reblock115・sct2・sing44・supportcase40・tbfp-network・uccase12)、P 7 問 (30n20b8・assign1-5-8・gen-ip002・gen-ip054・mas74・mzzv42z・noswot)。
- P (60 秒) で 6 秒の時点を見る価値のある問題は少ない (P の終わりの主のギャップ > 1% は neos-1456979・cvs16r128-89・supportcase26・blp-ar98・
  bppc4-08・ran14x18-disj-8 だけ)。
- 読み: 解なしの 13 問は「最初の解」が問題 (目的値の目標は効かない)。解はあるが悪い問題 (最初の解が数秒で出て、良くならない) が目標の対象。

## 4. 前処理の間の空きコア (事実、4 スレッド、`ENOMOTO_MIP_DEBUG_PRETIME=stop`、`pretime_cores.txt`)

段ごとの `秒/回/コア` (コア = プロセス全体の CPU 時間 / 経過時間。0.3 秒以上の段だけ):

| 問題 | 前処理の秒 | 平均のコア | 長い段 |
|---|---|---|---|
| rail01 | 12.7 | 1.19 | probe 16.8 秒 / 1.13 (入れ子を含む)、clqext 1.0 / 1.00 |
| academictimetablesmall | 14.7 | 1.94 | probe@re 15.0 / 2.01、lpprobe@re 2.0 / 2.76 |
| satellites2-60-fs | 9.9 | 2.18 | probe 11.1 / 1.80、lpprobe 2.8 / 3.11 |
| sorrell3 | 25.3 | 1.37 | **clqext 18.4 / 1.00**、probe 7.2 / 3.44 |
| opm2-z10-s4 | 8.8 | 2.86 | lpprobe 5.2 / 3.28、probe 3.9 / 3.38 |
| neos-3216931-puriri | 9.2 | 2.14 | lpprobe 5.8 / 1.95 |
| trento1 | 3.8 | 1.98 | lpprobe 3.3 / 2.03 |
| neos-5188808-nattai | 1.0 | 1.92 | lpprobe 0.7 / 2.04 |

- 前処理の間、4 本のうち平均 1.2〜2.9 本しか働いていない (空きは 1.1〜2.8 本)。並列のプロービング (probe・lpprobe) でも 1.1〜3.4 本。
  1 本だけの段: クリーク行の拡張 (clqext、sorrell3 で 18.4 秒)・対称性 (sym)・簡約 (reduce)。
