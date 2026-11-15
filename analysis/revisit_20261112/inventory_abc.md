# 見送った試作の表 (分類 a〜c、inventory.tsv から作った)

分類: a = 効き目が小さい・揺れの範囲、b = 区間が境目、c = 当時の既定との組み合わせの害。d・x は inventory.tsv だけに載せた。
「印」: 測り直しの設定名 (cfg.py) / 受け持ちの課題名 / src に無い (環境変数が今のソースにない)。数字は記録の言葉を補助のエージェントが写したもの (推測で埋めていない)。

| 分類 | 試作 | 記録 | 当時の数字 | 見送った理由 | 印 |
|---|---|---|---|---|---|
| b | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1` | cbcimpl_20261029 | 58問×2: +3.0% (40問+5.4%)・81 | 揺れの範囲 |  |
| b | `MIP_CMIRDELTA=1` | cmirdelta_20261107 | 58問×種1・2: SGM 15.50→15.61 (+0.7%、95%区間[−4.9%,+6.3%]、p=0.82)、解けた86→84; 根の時間×1.067; neos-3083819-nubu 8.0→2.2秒、drayage-25-23 14.0→3.9秒; exp-1-500-5-5 4.0/3.7→7.0/ | 既定にはしない、58問の平均が動かず根が少し遅くなる |  |
| b | `MIP_CUTCONF=b` | cutconf_20261110 | 58問: +1.3% (区間[−2.1%,+5.1%])、解けた82→79 | 効き目が見えない |  |
| b | `MIP_NO_ZEROHALF=0` | cutmix_20261111 | 58問×種1・2: SGM +1.1% (区間[−1.8%,+4.4%]); 10teams・noswotはどの種でも速く、misc07・eil33-2・neos-860300はどの種でも遅い | 得失が型の割合と相関しない (/ρ/≦0.24) |  |
| b | `MIP_FPR=1` | fpr_20261110 | 58問×種1・2 (直した版): SGM 18.64→18.93 (+1.6%、95%区間[−1.8%,+5.4%])、解けた81→82、最初の解の時刻は有意に早い(p=0.004); neos-1456979 最初の解 60秒前後→1〜2秒; 最初の版は+1.5% [−4.2%,+7.8%]・10teams揃って遅い | 既定にする価値は今はない、FJ+Local-MIPのスレッドと役が重なる |  |
| b | `MIP_ROBUST_ROOTIPM=1+MIP_ROBUST_STALL=rw+MIP_ROBUST_LNSFREQ=1+MIP_ROBUST_IMPORT=1+MIP_MEMBQ_LNSGROW=1` | fragility_20261105 | 600秒27問1回: 解けた def 4→rob5 3 (mzzv42z 346秒→時間切れ); neos-1171448 65→37秒、fiball・neos-933966・neos-1456979のギャップ縮む; widden 8.8e-2→4.0e-1、swath3悪い | 5つまとめては既定にしない、1回ずつで揺れを含む |  |
| b | `MIP_GAP4=procl` | gap4_rootstuck_mem_20261109 | 58問×種1・2: SGM −1.2% (95%区間[−7.5%,+6.2%])、解けた86→86、最大RSSの和−14.6%; 揃って遅い physiciansched6-2 21→52秒 (p)、roll3000 21→55秒・nw04 13→25秒 (l) | 揃って遅い3問の元はpとl |  |
| b | `MIP_GAP4=procln` | gap4_rootstuck_mem_20261109 | 58問: procl→procln +0.9% (区間[−3.5%,+6.5%])、86→86; 600秒の最大RSS 5問で−27〜−56% (rocln) | nはwiddenでノードのLPが1.6倍重い兆し (1回) |  |
| b | `MIP_MC11FIX2=a` | mc11_wait2_20261103 | 58問1回: 解けた 44→44、SGM(+10) 14.50→14.92 (+2.9%)、timtab1 48.7→時間切れ。mc11 4回 SGM 15.5→16.1。hypothyroid-k1 29.0→34.7、neos-911970 8.5→12.8 | 下界はほとんど上がらず、重い分離の問題で遅い。採らない |  |
| b | `MIP_MC11FIX2=b` | mc11_wait2_20261103 | 58問1回: 解けた 44→45、SGM +1.9% (h80x6320d 時間切れ→38.9、mas76 7.5→12.6、noswot 12.5→25.9)。hypothyroid-k1 29.0→21.4 | 58問 +1.9%、今は既定にしない。1回ずつで揺れの中かも。GAP_EPS を 0.003 などに下げて58問x3回で確かめる案 |  |
| b | `MIP_MC11FIX2=ab` | mc11_wait2_20261103 | 58問1回: 解けた 44→45、SGM +3.8% | a は採らない |  |
| b | `MIP_LP_PROBE_AFTER_EXT=s` | lp_probe_after_ext_20261102 | 58問2回: SGM 17.13→17.87 (+2.7%)、解けた 87→86。neos-960392・physiciansched6-2 が負け、neos-827175 5.6→3.8秒、noswot 10→37・27秒 | 既定にはしない (根の頂点が変わる揺れ) |  |
| b | `MIP_GR_RESTART=1` | mzzv42z_r4_20261021 | mzzv42z 60秒 GRN 3回 -20420・-20500・-20520 (既定 -18472〜-20380)。後の対の計測: 40問 26・19.65→26・20.17 / 26・20.01→26・19.40、16問 12→10 (mc11・roll3000 を落とす) / 9→11 | 一勝一敗で GRa の16問で2減るので不採用 (improvement_history) |  |
| b | `MIP_FC_CUT_ROUNDS=1` | mc11_r3_20261020 | mc11 60秒下界 11489・11509→11610・11598.5、120秒でも解けない。p200x1188c 7.1・8.7→5.4・5.6秒。後の対の計測: 40問 26→26 x2、SGM 20.17→20.58 / 18.22→18.36、16問 SGM 34.06→37.33 / 32.88→32.83 | 対象の問題は解けず SGM も悪い、既定にしない (improvement_history 2026-10-21) | src に無い |
| b | `MIP_HNFCUT=bc` | hnfcut_20261107 | 58問60秒2回: +1.7% (区間 [−1.5%, +5.5%])、解けた 85→81 (落ちた noswot などは bc が何もしない問題で揺れ)。avoca 300秒時間切れx2→5.6・9.3秒、rococoB10 600秒下界 11245・11313→14107・14122 | 既定にはまだ入れない (当たる問題が collection でも6問と少ない) | hnfbc |
| b | `SIMPLEX_LUCLOCK=b` | luclock_20261111 | 58問x種1〜2 (115組): 対の比 +2.9% [−0.3%, +6.6%]、解けた回 82→81。satellites2-60-fs で共有の下界による道の揺れが消える | 取り込みの基準を満たさない。58問では速さの得がない | (11-11 に測った) |
| b | `MIP_CUT_LP_HALVE+T_MIP_CUT_LP_CAP_MULT=3+MIP_CUT_CAP_FROM_LOOP+MIP_SUBMIP_HIGHS=1` | halve_submip_20261030 | hm3_cfl+h1 58問2回: SGM 20.31→21.06・20.35→20.62 (+3.7%・+1.3%)、解けた 38→41・37→40。mc11 2回とも落ちる | 既定にする候補 (mc11 は h1 の側)。後の bench で hs 形へ |  |
| b | `MIP_DIVE_LOCK_ORDER=1+T_MIP_NOINC_RENS_RATE=0.6` | halve_submip_20261030 | lock06 58問2回: +2.7%、82→84。Mittelmann +0.1% (47→46)。physiciansched6-2 3/3→2/3 | 既定にしない |  |
| b | `MIP_DIVE_LOCK_ORDER=1+MIP_RENS_HIGHS=1` | halve_submip_20261030 | rhlock 58問2回: +3.5%、82→84。Mittelmann +1.3% (47→45)。physiciansched6-2 3/3→1/3 | 既定にしない |  |
| b | `MIP_BG_RETURN_DSE=1` | halve_submip_20261030 | bgret 58問2回: +3.2%・+7.3% (全回 +5.2%、86→84)。Mittelmann −0.5% (47→48) | 得が揺れを超えない、採らない |  |
| b | `MIP_LOSE6_3_JOINSTOP=1+MIP_LOSE6_3_CUTRESTART=hw+T_MIP_LOSE6_3_CUTRESTART_FRAC=0.3` | halve_submip_20261030 | l3 58問2回: −3.1% (85→86)、Mittelmann +0.9% (48→48)、片方以上解 +4.8%。drayage-25-23 8.7〜10.1→6.5〜8.5、irp 18.4→38.4、neos-1582420 34.8→52.0 | 単独では採らない (irp・neos-1582420) |  |
| b | `MIP_ROBUST_ROOTIPM=1+MIP_ROBUST_STALL=rw+MIP_ROBUST_LNSFREQ=1+MIP_ROBUST_IMPORT=1+MIP_MEMBQ_LNSGROW=1` | halve_submip_20261030 | rob5 58問2回: +1.9% (86→85)、Mittelmann +0.8% (49→48)、片方以上解 +4.4% | ±2% の揺れの中、効き目も害もはっきりしない (bench_method ではわずかに損の側) |  |
| b | `MIP_ROBUST_ROOTIPM=1` | halve_submip_20261030 | rootipm 58問2回: +0.4% (86→85)、Mittelmann +1.1% (49→47)、片方以上解 +5.5% | 揺れの中 |  |
| b | `MIP_SUBA_RATE=n` | halve_submip_20261030 | suban 58問 種1〜3: 15.71→16.05 (+2.2%)、解けた 128→128。drayage-25-23 3回中2回時間切れ (4.8・4.3・53.0→60・5.6・60) | 揺れの中だが悪い側、drayage-25-23 の害。既定化しない |  |
| b | `MIP_NODECNT=hr` | nodecnt_20261107 | 58 問 60 秒 種 1・2: SGM 17.51 → 17.78 (+1.5%、95% 区間 −1.3%〜+4.9%)、解けた 84 → 84。遅い回 drayage-25-23 種 1 6.1 → 22.4、noswot 種 1 29.5 → 49.2 (種 2 は逆)。600 秒 kasai 種 1 最適 55 | 「既定にはしない」(効く問題が 1 問、58 問の差はまだ + 側) | TREEEFF |
| b | `MIP_OPTFACE_PARCUTS=3kx` | optface_cuts_20261105 | 58 問 種 1: 解けた 43 → 42、SGM 10.42 → 11.06、主ギャップ 4.24% → 5.35% (mc11・noswot 時間切れ)。beasleyC3 中央値 14.8・最悪 21.2 (全設定で最良)、neos859080 中央値 37.4・最悪 167.6 | 58 問で既定より良くない |  |
| b | `MIP_REDSPLIT=b` | redsplit_20261107 | 58 問 60 秒 種 1・2: SGM 16.59 → 16.64 (+0.4%、95% 区間 −4.4%〜+5.3%)、揺れの大きい 12 問を除くと +5.2% (区間 +1.1%〜+10.0%、p=0.03)、解けた 84 → 84、根の下界のずれ −0.115 (p=0.026)。2 回とも遅い qnet1  | 揺れの小さい問題で 5% 遅い (根の縮約とカットの時間) |  |
| b | `MIP_ROOTSTOP=r` | rootstop_20261111 | 58問×種2: SGM 15.80 (+0.1%)、95%区間 [−7.5%, +9.9%]、揺れの大きい12問除き −1.4% [−3.8%, +1.0%]、解けた 87→88。揃った差 10teams +1.24×2 (根は同じ、木の運)・noswot −0.65×2。時間切れの最終ギャップやや悪い (+0.049 | 区間の上限が +5% を越え基準を満たさない | cand1112 |
| b | `MIP_ROOTSTOP=rb+T_MIP_ROOTSTOP_RATE=0.02` | rootstop_20261111 | 58問×種2: SGM 15.62 (−1.1%)、95%区間 [−8.3%, +6.9%]、12問除き −2.9% [−5.8%, −0.1%]、解けた 87→88。揃った差 10teams +0.83×2・noswot −0.78×2。10teams 除くと −3.4% [−9.4%, +2.5%] | 全問の区間の上限が +5% を越える。時間切れの最終ギャップがやや悪い向き | cand1112 |
| c | `MIP_BASIS_NB=b` | basis_nb_20261102 | 写しの根LP physiciansched6-2 7486→1反復・neos-1171448 3540→1; h80x6320d 37.7→53.6秒・3/4、mc11 25.7→33.0 | 条件なしではh80x6320dで損(道が変わる) |  |
| c | `MIP_BG_CUTS_DSE=r+MIP_BG_CUTS_LP_CAP=1` | bgdse_20261102 | 6問とも写しLPの失敗0 (1回ずつ); 遅い機械の真似physiciansched6-2 2/2最適 (23.9・24.6秒) | bench16に勧めた形 (=rの部分は後でbench16で害) |  |
| c | `MIP_TAB_NNZ_CBC=1` | cbcimpl_20261029 | 1段目 21問−8.2%・40問−5.1%; 5段目58問×2 −2.6%・解けた81→87; Mittelmann −1.7%・41→46 (+7−2); 再計測 Mittelmann −0.9%・58問−0.3%・80→79 (+3−4); mc11・ns1208400が解けなくなる | このまま既定にはしない、根の予算で問題により大きく得・損 (mc11・ns1208400・30n20b8の根の下界) |  |
| c | `SIMPLEX_SCALE=1+T_SIMPLEX_SCALE_MIN_M=10000+T_SIMPLEX_SCALE_MIN_RANGE=1e4+SIMPLEX_HARRIS2=r+SIMPLEX_NOISE_RESE` | combb_600_20261106 | 600秒12問×2回: momentum1 根のLPが終わらない→14.6・14.8秒(ギャップ72.2%・69.4%)、satellites2-40 根151・154→97・96秒; supportcase42 根の終わり32→80・87秒 (RINS 5.5→54秒) | HARRIS2=rとNOISE_RESETの組み合わせでsupportcase42に害、形はcombではなくcomb |  |
| c | `SIMPLEX_NOISE_RESET=0.1+SIMPLEX_NOISE_RESET_W=e` | combb_600_20261106 | nronly: momentum1・satellites2-40に効かない; timtab1でも働かず | MIPの中では得が見えず、supportcase42の害の片方の原因 |  |
| c | `MIP_CLIQUE_CUTS=1` | drayage_20261020 | drayage-25-23 60秒で解けない→18〜21秒 (=1/=root 6回中5回)、drayage-100-23 27.7→8〜10秒; 副作用 misc07 6.9・7.6→10.5・10.2秒、air03 1.86→2.42秒; 10teams 59.4→11.9 | このままでは既定にしない (misc07が遅い) | CLIQFB |
| c | `MIP_CLIQUE_CUTS=root` | drayage_20261020 | misc07 10.7・10.0秒 (既定6.9・7.6) | このままでは既定にしない | CLIQFB |
| c | `MIP_CLIQUE_CUTS=auto` | drayage_20261020 | 40問中8問で使う: neos-860300 27.3・28.3→21.6・21.3、dcmulti 1.33・1.12→0.91・0.82; 30n20b8 402→解なし×2、timtab1 下界653261→641760、beavma 0.76→1.58秒 | 30n20b8が解なし、閾値でdrayageと30n20b8・timtab1を分けられない | CLIQFB |
| c | `MIP_NO_ZEROHALF=0` | gap3_zhtrial_20261109 | 13問の比0.83; hypothyroid-k1 68→30秒、comp07-2idx 394→23秒; neos-860300 14→31〜36秒、misc07・eil33-2・ns1830653が遅い | 効く問題と害のある問題を根の指標で見分けられない |  |
| c | `MIP_NODE_CUT_ADAPT` | highsonly16_summary_20261020 | neos-911970 時間切れ→3回中2回 49〜58秒。binkar10_1・10teams 良、rout +3〜6秒・misc07 +1.5秒 | 保留: qiu を2回とも時間切れ (密なカットでノードの LP が重い)。密度上限・LP予算を足したが今の既定との対の計測がない |  |
| c | `MIP_NODE_CUT_ADAPT=1` | neos-911970_20261020 | neos-911970 60秒: 既定 時間切れ3回→58.1・48.8秒で最適・1回時間切れ。打ち切り値あり 51.3秒x2。binkar10_1 28.5・23.3→13.1・11.2、10teams 36.6→24.1、misc07 7.9・7.5→9.2・9.4 (約20%遅い)、rout 9.1・9.0→15 | 候補にはなるが rout・misc07 が遅く neos-911970 も境目。採否は40問の全体の計測で |  |
| c | `MIP_IMPLBD_GENINT=1+T_MIP_CUT_LP_RETRY=4+MIP_FACE_SEP=off` | mzzv42z_r3_20261020 | G+F: mzzv42z 根の下界 -21246→-20852〜-20969、60秒暫定解 -20090〜-20540 (8回、R 付き) | G は副作用 (neos-1456979・roll3000 悪化気味)、F は40問で測らないと判断できない |  |
| c | `MIP_TREE_ROOT_BASIS=1` | mzzv42z_r4_20261021 | neos-911970 45・54→23・30秒、beasleyC3 TB 29〜51秒と時間切れ1回、10teams・misc07 1〜2秒遅い。GR と組 (F) で mzzv42z -20430〜-20520、下界5回とも有限 | 「従が根の LP で時間切れなら下界 +inf」は切り出して既定にしてよい候補。基底の受け渡しは40問・16問の対の計測で決める |  |
| c | `MIP_ROOT_CUT_HIGHS=1+T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000+MIP_CUTSEL_HIGHS=1+MIP_CUTSEL_HIGHS_POOL=1+T_MIP_ROOT_` | highstree_phys_20261024 | POOL 60秒2回: h80x6320d 58→26秒、mzzv42z 最適値、neos-960392 -35→-223、app1-1 21→15秒。計測14で noswot・mc11 を落とし hypothyroid-k1・dcmulti 遅い。計測16 (9問x4回): 最適 22→16 | 得と損が問題で分かれる。mc11 (固定費用型で fc_relax を上書き)・hypothyroid-k1 (密なカットを落とす) の損 |  |
| c | `MIP_RESTORE_READD=3` | node_restore_20261021 | 4 スレッド: 共有の待ち行列のノード 1300〜6500 → 150〜290 反復。mc11 (打ち切り 90 秒) 下界 11557・11591 → 11642・11662、neos-2978193-inde 既定 4/6 → 4/4、10teams 37.6・時間切れ → 15.1・21.3 秒。悪い: qiu  | 「既定にする候補」と書いたが 40 問全体は見ていない (その後の採否の記録なし) | src に無い |
| c | `MIP_PROBE_EQUIV2=1+MIP_ROOT_CUT_HIGHS=1+T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000+MIP_CUTSEL_HIGHS=1+MIP_CUTSEL_HIGHS` | phys_equiv_20261025 | 10 問 4 スレッド 60 秒 2 回 (pecce): physiciansched6-2 4/4 最適 (20.6〜45.9 秒、既定は前回まで 4/4 時間切れ)、h80x6320d 51.4・55.2 → 26.7・27.4、roll3000 時間切れ → 47.6・47.8。損: noswot 31.5・2 | POOL の損 (noswot・hypothyroid-k1) と同じ問題 → CUTSEL_PROBE で問題ごとに選ぶ形へ |  |
| c | `MIP_PROBE_CLIQUE=1+MIP_PROBE_REPRESOLVE=1` | physiciansched_r3_20261020 | p5s との比べ 60 秒: 30n20b8 解なし ×2 → 553 ×2、neos-860300 27.3 → 25.4、blp-ar98 6406.36・6443.63 → 7030.63 ×2 (悪い)、physiciansched6-2 暫定解 49424〜49524 → 51629 | 「直す価値が高いが、このままでは既定の候補にしない」(blp-ar98 の悪化、カットの予算なしでは効かない) |  |
| c | `MIP_TREE1_RESTART=1+MIP_RESTART_ROOT_CUTS=1` | restart_root_20261104 | 600 秒 1 回: kasai 最後の根 972850 → 975770、返す下界 986426 → 987010。swath3 根 +0.84、解けるまで 395.9 → 465.7 秒 | 揺れの範囲。木の中の再スタート自体が既定でない |  |
| c | `MIP_SUBMIP_HIGHS=1 (h1)` | submip_20261024 | 60秒6問×2回: physiciansched6-2 49424×2、mzzv42z −20450、neos-960392 最適×2。20秒56問: 最適 20→20、時間の和 851→870、サブ MIP の時間 201.5→624.4秒、nw04 24.2・nursesched 25.7秒で上限超え | 根のサブ MIP の時間が3倍になり上限を4〜6秒超える。候補にしない |  |
| c | `T_MIP_SUBMIP_NOSOL_CAP=250+MIP_SUBMIP_NOSOL_CAP_SERIAL=1+MIP_RENS_MORE_NODES_REAL=1 (ns250s)` | submip_nodes_20261101 | 9問×6回: n5-3 49.6→36.1、neos-1582420 31.0→30.0、h80x6320d 22.8→22.7、physiciansched6-2 6回とも解ける (既定 4/6)。h80x6320d で 9回中2回 6386.099 を誤って最適 (前処理の誤りの道を開く) | 誤った最適値の道を開くので、前処理の誤りを直すまで既定は変えない。58問・Mittelmann は未測定 | ns250s |
| c | `MIP_LP_INT_BOUNDS=1+MIP_PROOF_KEEP_INT=1+MIP_PROOF_CMIR=1+MIP_PROOF_CMIR_POOL=nocap+MIP_CUTPOOL_PROP=1 (kpnf)` | tree_replay_20261022 | neos-2978193-inde 3/3 最適 (59.2・40.2・44.2秒)。40問1回 25→23、pk1・10teams 落とす、qiu・rout・misc07・neos-860300 遅い | LP_INT_BOUNDS か CUTPOOL_PROP の副作用 (切り分けていない) |  |
| a | `MIP_CUT_INTOBJ_STALL=1+MIP_OBJ_STEP_SKIP_FIXED=1` | 10teams_20261023 | 10teams 5.7・18.2・15.0秒 (INTOBJ_STALL単独5.4・10.9・5.4) | 10teamsで効果なし、既定にする候補ではない |  |
| a | `MIP_SUBMIP_CUTS=1` | 10teams_20261023 | 10teams 28.2・10.1秒 (既定10.1・60・38.4)、根サブMIPで924 1/2 | 効かない(1回はRRCが始まらず) |  |
| a | `T_MIP_SUBMIP_TIME_FRAC=0.15` | 10teams_20261023 | 10teams 8.5・9.3秒 (2/2で根で924) | 根で待つ時間が延び全問題に影響 |  |
| a | `MIP_NO_LPFACE=1+T_MIP_RRC_TIME_FRAC=0.14` | 10teams_20261023 | 10teams 9.9・9.6・9.1秒 (3/3) | サブMIPの時間を延ばすと全問題に影響 |  |
| a | `T_MIP_CUT_STALL=3` | 10teams_20261023 | 10teams 6.1・5.1・5.9・5.1・6.3秒 (5/5) | 全問題で停滞3回になる(INTOBJ_STALLの元) |  |
| a | `MIP_BASIS_NB=s` | basis_nb_20261102 | サブMIP根LP反復1.04倍、時間1.06倍; physiciansched6-2 0/2 | 効かない(希望で変えた数0〜5) |  |
| a | `MIP_BASIS_NB=r` | basis_nb_20261102 | 再スタートのある3問で後の根LP反復変わらず、時間1.031倍 | 効かない |  |
| a | `MIP_BASIS_NB=w` | basis_nb_20261102 | neos-1171448 3/4・50.1秒、physiciansched6-2 2/4・40.5秒、h80x6320d 4/4・38.9秒、mc11 32.0秒 (4回); tw: neos-1171448 4/4・44.2 | 58問で確かめる候補 (BG_CUTS_BASIS_X=lpwと同じ効き) |  |
| a | `MIP_NODE_CUTS+T_MIP_NODE_PATH_STARTS=1000` | beasleyC3_mc11_20261019 | beasleyC3 57.3秒で最適、mc11 下界11484.3 (1回) | 後で同じ設定で2回とも時間切れ(揺れの当たり) |  |
| a | `T_MIP_NODE_CUT_FREQ=3` | beasleyC3_mc11_20261019 | beasleyC3 時間切れ下界752.2、mc11 11485.1 (1回) | 解けない |  |
| a | `MIP_FC_NODE_SEP_ADAPT=1` | beasleyC3_mc11_20261020 | beasleyC3 下界746.8 (時間切れ)、mc11 11457.7; 分離4〜5回だけ | 分離がほとんど走らない、方向を捨てた |  |
| a | `MIP_FC_NODE_SEP2=1` | beasleyC3_mc11_20261020 | 判定に当たる20問×2回: 解けた12/11→13/12、SGM 20.57/21.48→19.48/21.24; beasleyC3 SEP2で5回中4回; h80x6320d +2〜5秒 | 既定では使わない(記録では「既定にする候補」、40問の全体は未計測) |  |
| a | `MIP_FC_NODE_SEP2=1+MIP_FC_NODE_PATH_HIGHS=1` | beasleyC3_mc11_20261020 | beasleyC3 25.2秒で最適(同程度)、mc11 下界11455.1(悪い) | mc11で悪い、SEP2に入れない |  |
| a | `MIP_FC_NODE_SEP2=1+T_MIP_CUT_AGE_LIMIT=50` | beasleyC3_mc11_20261020 | mc11 LPの行は減らず(3770)、下界11485.5 | 効かない |  |
| a | `T_MIP_SYM_TIME_EXT=1` | beasleyC3_r3_20261020 | beasleyC3 ext・ext2・p5sext 8/14 (既定5/6・p5s 3/4); 最初の版で nw04 11.4・10.4→26.0・25.7秒、h80x6320d 34.4・38.5→40.4・48.0秒 | 対称性の揺れを消すだけで解ける割合は上がらない、既定の候補ではない |  |
| a | `MIP_BG_CUTS_REHEUR=rens` | beasleyC3_r3_20261020 | beasleyC3 根の暫定解758・763・765、3回中2回(38.2・45.4秒) | 756に届かず決め手にならない |  |
| a | `MIP_BG_CUTS_REHEUR=rins` | beasleyC3_r3_20261020 | beasleyC3 4/4、21.8〜54.5秒、暫定解762〜774 | 決め手にならない |  |
| a | `MIP_RENS_DIVE=1+MIP_RINS_DIVE=1` | beasleyC3_r3_20261020 | beasleyC3 3/3、19.6〜45.6秒、根の暫定解773〜780 | 決め手にならない |  |
| a | `MIP_BG_CUTS=heavy` | beasleyC3_r3_20261020 | beasleyC3 根741.96→742.27・742.39、4/4・24.8〜40.4秒 | 根がほぼ上がらない |  |
| a | `MIP_SUBMIP_CUTS=1` | beasleyC3_r3_20261020 | beasleyC3 8回中7回、時間の中央値31.8秒(既定11回中8回・38.1秒) | 10/16に20問で不採用 |  |
| a | `MIP_FC_SUBMIP=1` | beasleyC3_r3_20261020 | beasleyC3 6/6 (既定8/11)、中央値32.4秒; r50x360 15.5・20.3→29.1・21.3、beasleyC2 10.1・9.5→15.7・10.1; mc11 11689到達 | 既定では使わない(記録では「既定にする候補」、副作用7問×2回だけ) |  |
| a | `T_MIP_RENS_RATE=0.516+MIP_SUBMIP_CUTS=1+MIP_BG_CUTS_REHEUR=rens` | beasleyC3_r3_20261020 | beasleyC3 3/3、根のRENS 778〜806、背景LPのRENS 758〜759 | 756に届かず |  |
| a | `MIP_DIVE_LOCK_ORDER=1` | beasleyC3_r3_20261020 | 近傍の最適値765・760 (乱数の順767・764) | ほぼ同じ、60秒の計測なし |  |
| a | `MIP_BG_CUTS_DETERM=r` | bg_determ_20261102 | qnet1・p200x1188cの待ち0.9〜1.0秒・0.2秒のまま; mc11で1回44ラウンドで止まる | 効かない、使わない(勧めない) |  |
| a | `MIP_BG_CUTS_LP_WORK=1h` | bg_lp_work_20261102 | 半分化はatlanta-ipの1回だけ; 遅い機械の真似のphysiciansched6-2 2/3 | ほとんど働かず得が見えない |  |
| a | `MIP_BG_CUTS_DSE=rw` | bg_lp_work_20261102 | 単独では写しの失敗を半分しか直さない(neos-960392 2/3、comp07-2idx 1・2ラウンド目が上限); 時間の上限の超過なし | 既定では使わない(LP_WORK=1と重ねた形が第2候補) |  |
| a | `MIP_BG_RETURN_DSE=1` | bg_return_dse_20261102 | 7問×3回 SGM(+1) 52.1→49.6秒; physiciansched6-2 1243〜6345→690反復; seymour1は3回とも暫定解411.402 (既定411.124〜411.177) | 既定にはしない、bench17で害がないか(seymour1)見てから |  |
| a | `MIP_BG_CUTS_DSE=1` | bgdse_20261102 | neos-960392 2/2で既定と同じく0・1ラウンド目が上限; atlanta-ip既定より悪い回; 遅い機械の真似1/3 | 効きが半端(主の重みのずれ) |  |
| a | `MIP_BG_CUTS_LP_CAP=1` | bgx_check1_20261102 | physiciansched6-2 6回: 6/6のまま (SGM(+1)26.6→26.6)、遅い機械の真似(0.6秒) 1/6→5/6; bgdseでは単独でneos-960392を直さない | 既定では使わない(58問で確かめて既定にする候補) |  |
| a | `MIP_LMIP_SWAP=1` | binheur_20261111 | 58問×種1〜2: SGM 15.81→15.72 (−0.6%、95%区間[−4.9%,+3.1%]、p=0.81)、解けた84→86、誤答・矛盾0; neos-911970は8対中5対で遅い | 効き目が小さく区間は0をまたぐ、取り込みは親が決める(自分では既定にしない) | cand1112 |
| a | `MIP_PRECHAIN=ebiR+MIP_LMIP_SWAP=1` | binheur_20261111 | 当たる9問×種1〜2: ebiR→ebiR+SWAP SGM +3.5% (区間[−4.2%,+15.1%])、解けた8→7 (neos-911970の種2が時間切れ、除くと−1.4%) | 重ねても効き目は大きくならない | cand1112 |
| a | `MIP_ROLE_LMIP=1+MIP_LMIP_SWAP=1` | binheur_20261111 | 当たる9問×種1〜2: ROLE_LMIP→+SWAP SGM +6.7% (区間[−0.2%,+20.5%])、解けた8→7 (neos-911970種2、除くと+0.4%) | 重ねても効き目は大きくならない | cand1112 |
| a | `MIP_BOUND_STEP=1+MIP_OBJ_STEP_IMPLINT=1` | bound_step_20261104 | 10問: 刻みのない9問は切り上がらず、30n20b8 bs 140・bsi 151 (刈り込みも変わり別の木) | 10問だけ見た、bppc4-08は切り上がらない |  |
| a | `MIP_RENS_MORE_NODES_REAL=1+MIP_SUBMIP_HIGHS_DEEP=-c+T_MIP_SUBMIP_HIGHS_TIME_MAX=8` | cand_slow_n5-3_20261031 | n5-3 33〜38秒(def並み); qiu 4回中2回遅い(54.6・60.0)、h80x6320d 1回37.6秒 | 得の問題を削る気配、回数が足りないので勧めない |  |
| a | `MIP_SUBMIP_HIGHS=-st` | cand_slow_neos-1582420_20261031 | neos-1582420 6回平均27.3秒 (旧def 32.4、cand 49.7); 遅い3問は全部戻り得も残る(h80x6320d 3回中1回38.5、mc11 +2.7秒) | 既定の候補、7問だけなので58問2回とMittelmannで確かめる |  |
| a | `MIP_SUBMIP_HIGHS_NOSOL=1` | cand_slow_neos-1582420_20261031 | neos-1582420 34.2秒に戻る; n5-3は3回とも時間切れ; physiciansched6-2 4回中3回(揺れ) | n5-3に効かない、tを抜く形より弱い、既定にしない |  |
| a | `MIP_EARLY_RESTART_JOIN=1` | cand_slow_neos-2978193-inde_20261031 | neos-2978193-inde cand 41.7(中央値37.9)→35.9(33.0、12回); 得の5問×3回で損なし (h80x6320d 1回21.8秒) | 既定では使わない、58問・Mittelmann未計測 |  |
| a | `MIP_TAB_NNZ_CBC=node` | cbcimpl_20261029 | 9問×2回: SGM 45.13 (8/18) 既定42.90 (9)、ほぼ既定と同じ | 切り分け用、効かない |  |
| a | `MIP_TAB_NNZ_CBC=1+T_MIP_TAB_NNZ_GAIN=1e-4` | cbcimpl_20261029 | 6問×2回 SGM 40.05 (10/18)、ns1208400 20→14秒; 58問×2 SGM 19.16→19.43 (+1.4%)・80→80 (+4−4)、Mittelmann 46.04→45.95 (−0.2%)・43→44; qiu 2回とも時間切れ (根LPの同時実行の勝ち負けと判明) | 既定にするほどの差はない (揺れの範囲) |  |
| a | `MIP_TAB_NNZ_CBC=1+T_MIP_TAB_NNZ_CUTCAP=200` | cbcimpl_20261029 | 6問×2回: SGM 44.12 (7/18)、ns1208400直らず | ns1208400を直せない |  |
| a | `MIP_NODE_CUT_MAXLEN=1` | cbcimpl_20261029 | 1段目 21問−1.7%・40問−6.1%; 5段目58問×2 −1.2%・解けた81→85 | 2回とも良い側だが差は小さい | ncm |
| a | `MIP_DIVE_RCFIX=1` | cbcimpl_20261029 | 1段目 −4.0%/−4.5%; 5段目 +1.2%・81→81; neos-911970回0時間切れ・noswot回1 57秒 | 1段目の良さが再現しない、既定にしない |  |
| a | `MIP_ORIGROW_TIGHT=1` | cbcimpl_20261029 | 1段目 (58問×2) 21問−4.1%・40問−1.8% | 揺れの幅(±4.5%)を明らかに超えない |  |
| a | `MIP_ZH_PRE=1` | cbcimpl_20261029 | 1段目 21問−2.6%・40問−2.3% | 揺れの範囲 |  |
| a | `MIP_COST_PRIORITY=1` | cbcimpl_20261029 | 1段目 21問−2.8%・40問−2.1% | 揺れの範囲 |  |
| a | `MIP_PROBE_STRENGTHEN=1` | cbcimpl_20261029 | 1段目 21問−3.9%・40問−0.3% | 揺れの範囲 |  |
| a | `MIP_INEQ_SINGLETON_ALL=1` | cbcimpl_20261029 | 1段目 21問−4.4%・40問+0.6% | 揺れの範囲 |  |
| a | `MIP_POOL_SELECTED_ONLY=1` | cbcimpl_20261029 | 1段目 21問−1.3%・40問−1.2% | 揺れの範囲 |  |
| a | `MIP_SEP_ADAPTIVE=1` | cbcimpl_20261029 | 1段目 21問−1.6%・40問+0.2% | 揺れの範囲 |  |
| a | `MIP_CUT_FAIL_RESTORE=1` | cbcimpl_20261029 | 1段目 21問−2.1%・40問+1.5% | 揺れの範囲 |  |
| a | `MIP_TIGHTEN_VUBS=1` | cbcimpl_20261029 | 1段目 21問+2.3%・40問−1.6% | 揺れの範囲 |  |
| a | `MIP_SB_FIX_RESOLVE=1` | cbcimpl_20261029 | 1段目 21問+0.1%・40問−1.0% | 揺れの範囲 |  |
| a | `MIP_PC_COUNT_CUTOFF=1` | cbcimpl_20261029 | 1段目 21問−0.9%・40問+0.2% | 揺れの範囲 |  |
| a | `MIP_ZIROUND_ADAPT=1` | cbcimpl_20261029 | 1段目 21問−0.4%・40問+0.7% | 揺れの範囲 |  |
| a | `MIP_SEP_PARALLEL=1+MIP_SEP_NO_ROWCLONE=1` | cbcimpl_20261029 | 1段目 21問+1.2%・40問−0.5% | 揺れの範囲 |  |
| a | `MIP_SB_OF=1` | cbcimpl_20261029 | 1段目 21問+1.6%・40問+1.2% | 悪い側、揺れの範囲 |  |
| a | `MIP_SB_TIGHTEN=1` | cbcimpl_20261029 | 1段目 21問+0.5%・40問+2.0%; 2段目58問 +2.5%・解けた81→79 | 悪い側、揺れの範囲 |  |
| a | `MIP_SB_TIGHTEN_INHERIT=1` | cbcimpl_20261029 | 58問×2: +1.1%・81→81; 21問+3.3%・40問+2.8%; neos17 13→8.5秒 | 揺れの範囲 |  |
| a | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1` | cbcimpl_20261029 | 58問×2: +2.1%・81 | 揺れの範囲 |  |
| a | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1+MIP_SB_POLY_RC=1` | cbcimpl_20261029 | 4段目58問 −1.2%・83; 5段目 −0.2%・82; 6段目58問×3 −1.3%・一般整数16問−4.9% (3回とも−4〜−6%); Mittelmann −1.2%・41→43; noswot 23〜60→18・12・18秒 | 58問全体では揺れの範囲、効く問題を集めた計測が要る | sbt |
| a | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1+MIP_SB_RC=1` | cbcimpl_20261029 | 5段目58問×2: +1.2%・解けた84 (既定81); neos-960392 両方の回で解けた | 全体では差が出ない |  |
| a | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1+MIP_SB_RC=1+MIP_SB_TIGHTEN_PROP=1` | cbcimpl_20261029 | 5段目58問×2: −0.2%・82 | 揺れの範囲 |  |
| a | `MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1+MIP_SB_POLY_RC=1+MIP_SB_RC=1+MIP_SB_POLY_S` | cbcimpl_20261029 | 6段目58問×3: −2.6%・16問−0.2%; Mittelmann −1.5%・41→43 | 揺れの範囲 |  |
| a | `MIP_TAB_NNZ_CBC=1+MIP_SB_TIGHTEN=1+MIP_SB_TIGHTEN_INHERIT=1+MIP_SB_TIGHTEN_ANCESTOR=1+MIP_SB_POLY_RC=1` | cbcimpl_20261029 | Mittelmann1回: SGM 46.17→45.27 (−1.9%)・41→43 (+4−2) | 解けた数がtab_nnz単独(46)より少ない | sbt |
| a | `MIP_CMIRDELTA=g` | cmirdelta_20261107 | 根の下界のずれ平均+0.008 (良い4・悪い7)、根の時間×1.013 | 単独では根の下界をほぼ変えない |  |
| a | `MIP_CMIRDELTA=c` | cmirdelta_20261107 | 効く問題(種3・4): drayage-25-23・qnet1でcdと同じ効き目 | 切り分け用 (連続分数が主と判明) |  |
| a | `MIP_CMIRDELTA=l` | cmirdelta_20261107 | 効く問題(種3・4): drayage-25-23・qnet1では効かない、nubuには効く | 切り分け用、LLLだけでは出ない |  |
| a | `T_PARTIAL_TAU_ADAPTIVE=1` | fable_kenn_netlib_20261003 | Kennington: ken-18 −22%、ken-13 −14%、pds-20 +2% | 対処案の候補 (作業#10で取り下げた手間比較版、採否はこの記録では未定) |  |
| a | `T_PARTIAL_TAU_MIN_M=1e9` | fable_kenn_netlib_20261003 | ken-18 −25% (部分tauなし) | 調べの実験、案のみ |  |
| a | `T_MAX_CONSECUTIVE_FILLIN_FAILURES=1000000` | fable_kenn_netlib_20261003 | ある問題で−20%; bnl2 前処理19→30ms、25fv47 反復+7% | 調べの実験、案のみ |  |
| a | `REUSE_PIVOT_ORDER=0` | fable_kenn_netlib_20261003 | 25fv47 −5%、bnl2 −7%、greenbea −5%、d2q06c −3%; 採用時はNetlib 93問で−1.5% | 全部切ると採用時の計測と逆、密な問題に限る案 (E) |  |
| a | `MIP_CUTCONF=1` | cutconf_20261110 | 58問×種1・2 (116対): SGM 18.58→18.50 (−0.4%、95%区間[−4.8%,+4.1%])、解けた82→82; 作った制約35.5万本・刈ったノード12.3万; 誤答0 (最初の版のMIR右辺の符号の誤りは直した) | 効き目が見えない、既定にする価値は今はない (害も小さい) |  |
| a | `MIP_CUTLPCAP=cl` | cutlpcap_20261107 | 根(20問、1回): widden 472.9→8794、thor50dday 4174→18189、neos-1354092 13.29→38.28、istanbul 55.14→66.48 | 一番効く(根)、60秒の58問は ac の形でだけ計測 |  |
| a | `MIP_CUTLPCAP=al` | cutlpcap_20261107 | 根(20問、1回): lectsched 8.694→14.667、thor50dday 4174→16168; MODKVBのphysiciansched6-2 48583→49274 | 本数の多い回だけ救う、cで同じかそれ以上 |  |
| a | `MIP_CUTLPCAP=nl` | cutlpcap_20261107 | 根: 100本でもまだ当たる(istanbul・lectsched); physiciansched6-2 49224→49074、sing44は0ラウンド目で当たるように | 効かない |  |
| a | `MIP_MODKVB=1+MIP_CUTLPCAP=al` | cutlpcap_20261107 | 600秒 physiciansched6-2: mk 373.0・21.9秒 → mka 34.3・34.0秒 | 決め手にならない |  |
| a | `MIP_CUTMIX=fh` | cutmix_20261111 | 16問×2種: SGM −1.4% (区間[−4.3%,+1.1%])、解けた21→21; mc11 種1 37.6→17.2、種2 35.7→47.5 | 58問×2種の結果は記録の時点で未記入 |  |
| a | `MIP_CUTMIX=c` | cutmix_20261111 | 計測なし (持ち上げつきの被覆自体が58問で+3.0%) | 割合で入れ切りする根拠がない、58問の計測なし |  |
| a | `MIP_CUTMIX=z` | cutmix_20261111 | 計測なし (zero-halfの得失は割合と相関しない) | 根拠がない、58問の計測なし |  |
| a | `MIP_CUTSEL_ENS=kd` | cutsel_ens_20261110 | 58問の種1(+種2の3回): +0.5% (区間[−5.7%,+7.1%])、解けた46→47、速い10・遅い10; (a)の10〜12問で+7.0% | 中立、効き目はない |  |
| a | `MIP_EQUIV2_HSEL=1+T_MIP_HSEL_AGE=0` | cutsel_highs_20261022 | physiciansched6-2 (e2allに足す): 下界49324に届く回2/6→3/4、1回42.2秒で最適 (10回中1回); 働くのは55問中physiciansched6-2だけ | P5(e2all)に足す候補、60秒で安定して解けはしない、e2allの判断とまとめて扱う | src に無い |
| a | `MIP_IMPLBD_SELECT=1` | cutsel_highs_20261022 | physiciansched6-2で採る本数も経過もほとんど同じ | 関係しない |  |
| a | `MIP_CUTSEL_VIOL=1+MIP_CUT_STALL_MOVE=1` | cutsel_restart_20261020 | 40問1組: 解けた25→24、SGM 21.99→21.86; 16問 5→5・47.39→46.30; noswot時間切れ(既定54.3)、neos-911970の下界53.46→52.32 | 既定にする根拠は足りない (1組だけ) | src に無い |
| a | `T_MIP_ROOT_CUT_AGE=10` | cutsel_restart_20261020 | neos-911970 根の下界44.87 (既定47.03) | むしろ下がる |  |
| a | `MIP_RESTART_CUTS_TO_POOL=1` | cutsel_restart_20261020 | 記録の表に数字なし | 比べた設定の一部 (結果の記載なし) |  |
| a | `MIP_SUBMIP_HIGHS=cgh+MIP_BG_CUTS_WAIT=1` | diag_gain_20261031 | 得の5問 SGM(+1) 15.7 (10/10)、損の4問 21.9→20.2 (hs_pi0_bgw比); neos17 23→16秒、binkar10_1 18秒で残る | 提案する最小の組み合わせ、2回だけなので58問で確かめるまで既定にしない |  |
| a | `MIP_SUBMIP_HIGHS=cgh+MIP_BG_CUTS_WAIT=1+MIP_SUBMIP_HIGHS_TRIM=r` | diag_gain_20261031 | 損の4問 SGM 17.4 (最小)、得の5問 18.2 (8/10); physiciansched6-2 0/2 | physiciansched6-2の得を失う |  |
| a | `T_MIP_SUBMIP_OUTCOME_MIN_NODES=150` | diag_pi0_20261031 | hs_pi0_bgwに足して: neos17 21→15秒、binkar10_1・h80x6320dの得は残る; 45問×2回 SGM 8.26 (86)→8.28 (85) | 全体の得にはならない、どれも既定にしない (neos17の損を消すならom150) |  |
| a | `T_MIP_SUBMIP_SB_TIME_FRAC=0.25` | diag_pi0_20261031 | neos17 13秒に戻るが binkar10_1 10→18秒、h80x6320dの得も一部消える; 45問×2回 SGM 8.30 (88) | binkar10_1で損、既定にしない |  |
| a | `T_MIP_SUBMIP_SB_TIME_FRAC=0.1` | diag_pi0_20261031 | 記録の表に結論の数字なし (sbf10) | 既定にしない |  |
| a | `MIP_SUBMIP_RELIABLE_INHERITED=1` | diag_pi0_20261031 | ほぼhsに戻り、得も消える | 既定にしない |  |
| a | `MIP_CUTSEL_DCD_PROD=1+T_CUTSEL_DCD_DENS_EXP=0` | dircutoff_20261022 | mik-250 7〜9・mc11 36〜43で悪化は小さいが 10teams 25〜26、beasleyC3 TL・TL、h80x6320d 47〜48 (既定37〜46) | 良さもない | src に無い |
| a | `MIP_CUTSEL_DCD_PROD=1` | dircutoff_20261022 | 1ラウンドの実験で α=0.5 は既定と同じ値に戻る (k=3: 11349) | 伸びが小さい | src に無い |
| a | `T_DISCARD_BAN_MULTI=1` | discard_cycle_20261102 | pilotnov (費用ずらし1e-8): 従来76.2秒→1.30秒 (上限50は0.74〜0.87秒)、両方1.44秒 | 通常の求解でも経路が変わり、この問題では打ち切りより遅いので既定にしない |  |
| a | `MIP_FOLD=1` | drcr_20261110 | 段階2 (連続列だけ) で畳める25問: SGM 1.008倍 | 段階2だけでは効き目がない |  |
| a | `LP_FOLD=1` | drcr_20261110 | MIPLIB 240問のLP緩和: 畳める99問; 畳める105問でSGM 0.76倍 (rfold)、cod105 12→0.01秒; 105問中26問で遅い (rail02 101→時間切れ) | LPのENOMOTO_LP_FOLDは既定にしていない (MIPの根には段階3の形でだけ) |  |
| a | `LP_FOLD=r` | drcr_20261110 | DRCRとほとんど同じ (畳める105問、SGM 0.76倍) | 反転の大きな差は再現しない、既定にしていない |  |
| a | `MIP_FOLD=riQq` | drcr2_20261111 | 32問×種1・2: SGM +0.7% (1.007)、20→20; neos-957323・neos-960392が数秒遅い | 差なし、得がない |  |
| a | `MIP_GAP1=u` | drcr2_20261111 | 既定の畳み+u (fu): 58問×種1・2 SGM 15.82→15.45 (−2.4%、95%区間[−5.5%,+1.4%]、p=0.205)、揺れの大きい12問を除くと−3.5% [−5.9%,−1.3%]、解けた86→86、誤答0; 7問300秒 SGM 0.960、neos-957323 6.5/9.3→4. | 取り込みの基準は満たすが既定にはしていない (判断は親・利用者)、働くのは2問だけ | cand1112 |
| a | `MIP_GAP1=u+MIP_FOLD_PROBE_INT=1` | drcr2_20261111 | 7問×2種300秒: SGM 0.989 (fuは0.960) | fuより良くない | cand1112 |
| a | `MIP_FJ_REPAIR=1` | fjrepair_20261023 | 12問×2回: neos-1456979 最初の暫定解 解なし・51.6秒→3.7〜3.8秒、最終216→224・210; 30n20b8は働くが解なし; 他は揺れの範囲 | 1問だけの効果、既定にするなら40問+16問を2組回してから | src に無い |
| a | `MIP_TREE_FJ=1` | fjrepair_20261023 | tfjonly: neos-1456979 235・48.6秒/312・50.5秒、30n20b8 755・39.3秒; 木のFJは一度も解を見つけない | 木のFJは既定にする候補ではない | src に無い |
| a | `MIP_ROBUST_ROOTIPM=1` | fragility_20261105 | neos859080 中央値162.6→3.4秒、最悪334→5.9秒、1スレッド600→8.6〜24.4秒; 58問では経路が変わらない | 既定では使わない (組み合わせてbenchにかける案) |  |
| a | `MIP_ROBUST_STALL=rw` | fragility_20261105 | neos-1171448 中央値61.3→44.4秒、最悪75.8→50.4秒; 58問(60秒)で害は見えない (60秒ではwは効かない) | 既定では使わない (組み合わせてbenchにかける案) |  |
| a | `MIP_ROBUST_LNSFREQ=1+MIP_ROBUST_IMPORT=1` | fragility_20261105 | beasleyC3 1スレッドで中央値・最悪とも約4割短い、4スレッドでは揺れの範囲 | 既定では使わない |  |
| a | `MIP_MEMBQ_LNSGROW=1` | fragility_20261105 | n2seq36q 遅い道で429〜578→192・243秒 (1回は効かず) | 既定では使わない |  |
| a | `MIP_ROBUST_TSTALL=1` | fragility_20261105 | mzzv42z 中央値・最悪とも良くならない | 呼ばれても近傍に改善がない |  |
| a | `MIP_FZ2_FIX=1` | fuzz_tiny_worse_20261103 | fuzz_tiny 400: HiGHSとの食い違い13→9、tolより悪い値を最適と返すのは5→0; cargo test 333通過 | 既定にするかはMIPLIBで前処理の結果が変わるかを見てから (正しさの直し) |  |
| a | `MIP_FZ2_FIX=n` | fuzz_tiny_worse_20261103 | 正規化の余裕だけで悪い5件すべてがtolに一致 | 切り分け用 (=1に含む) |  |
| a | `MIP_FZ2_FIX=m` | fuzz_tiny_worse_20261103 | 併合した整数列の上限の丸めだけ: 効き目は見えない | 効かない |  |
| a | `MIP_GAP1=u` | gap1_tree_20261109 | neos-957323 300秒: 種2は130秒・0ノードで最適、種1は時間切れだが残るギャップ1.66e-4→1.7e-5 (既定は2回とも時間切れ) | 記録は途中 (害の58問は未計測) | cand1112 |
| a | `MIP_GAP1=r` | gap1_tree_20261109 | neos-957323: 下界変わらず (-237.75684701)、開いたノード1万→2317 | 木の下界を動かさない | cand1112 |
| a | `MIP_GAP1=q` | gap1_tree_20261109 | neos-957323: 下界変わらず、並びを変えたのは157回 | 木の下界を動かさない | cand1112 |
| a | `MIP_GAP1=f` | gap1_tree_20261109 | neos-957323: 下界変わらず、ノード3108 (LP1回279反復で遅い) | 木の下界を動かさない | cand1112 |
| a | `MIP_GAP1=s` | gap1_tree_20261109 | neos-957323: 下界変わらず | 選ぶ列の分布は変わらない | cand1112 |
| a | `MIP_GAP1=pt` | gap1_tree_20261109 | neos-957323: 下界変わらず、証明で刈る27→166 | 木の下界を動かさない | cand1112 |
| a | `T_MIP_CONFLICT_WEIGHT=0.01` | gap1_tree_20261109 | neos-957323: 下界変わらず | 効かない |  |
| a | `MIP_CHILD_HIGHS=1` | gap1_tree_20261109 | neos-957323: 下界変わらず | 効かない |  |
| a | `MIP_TREE1_POOL=1` | gap1_tree_20261109 | 300秒種1: kasai 下界983527→984975、tr12-30 130492→130513 | 記録は途中 (害の58問・600秒は未計測) | TREEEFF |
| a | `MIP_TREE1_POOL=1+T_MIP_BB_EVERY_INC=2` | gap1_tree_20261109 | 300秒種1: 残りのギャップ kasai 49%・tr12-30 24% 縮む (下界985991・130517); LP1回の反復が2倍 | 記録は途中 (害の58問・600秒は未計測) | TREEEFF |
| a | `MIP_GAP2_FLAT=3+MIP_GAP2_ZEROCAP=2+MIP_GAP2_PROBE1=1+MIP_GAP2_LPPGIVE=0.5` | gap2_rootcost_20261109 | 4スレッド・種1・2: cod105 192→143・142 (HiGHS 131〜205); neos-960392 既定33〜98→56・68 (揺れで判定できない); physiciansched6-2 25.4・26.3→24.9・23.9 | 58問の害は計測中 (記録の時点で未記入) |  |
| a | `MIP_GAP2_FLAT=3+MIP_GAP2_ZEROCAP=2+MIP_GAP2_PROBE1=1+MIP_GAP2_LPPGIVE=0.5+MIP_LOSE6_1_STOP=w+T_MIP_LOSE6_1_BGW` | gap2_rootcost_20261109 | neos8 6.5・6.7→4.4・4.0、p200x1188c 4.3・7.6→2.8・4.0; neos-960392 52.1・60.6 | 58問の害は計測中 | prepar |
| a | `MIP_GAP2_ZEROCAP=2` | gap2_rootcost_20261109 | neos-827175 7.0・7.9→6.1・4.9; cod105のカット57→23〜30秒; neos-960392のカットのループ13〜58→5秒 (全体は揺れ) | 壁時計で打ち切るので、手間で数える形にして58問・186問で害を見てから既定の候補 |  |
| a | `MIP_GAP2_LPPGIVE=0.5` | gap2_rootcost_20261109 | cod105 LPプロービング30.4→20.3秒 | 単独の判断なし (g2の一部) |  |
| a | `MIP_GAP2_FLAT=3` | gap2_rootcost_20261109 | physiciansched6-2 25.1・23.4: ループは2.7秒早く終わるが内点法の丸めが最適解を出さなくなる | 効かない |  |
| a | `MIP_GAP2_PROBE1=1` | gap2_rootcost_20261109 | neos-960392 55.2・36.9: 2回目の比べ(約6秒)は消えるが揺れに埋もれる | 効かない |  |
| a | `MIP_CUTSEL_PROBE_KEEP=1` | gap2_rootcost_20261109 | physiciansched6-2 27.1・26.7: 49323.67で足踏み、ループが18.4秒まで延びる | 効かない |  |
| a | `MIP_LOSE6_1_STOP=w+T_MIP_LOSE6_1_BGWAIT=0` | gap2_rootcost_20261109 | p200x1188c 4.3・7.6→3.0・4.2 (裏のカットの待ち2.1・4.2→1.0・0.3秒) | 単独の判断なし (既存の切り替え) | ERWAIT |
| a | `MIP_PREPAR=ple` | gap2_rootcost_20261109 | neos8 6.5・6.7→4.2・4.3 (HiGHS 4.3・4.8) | 害の小さいことは分かっている (prepar_20261107)、既定にする判断を別に行う | prepar |
| a | `MIP_ZH_TRIAL=1` | gap3_zhtrial_20261109 | 13問すべてで使わないと判定 (既定と同じ動き) | 見分けにならない |  |
| a | `MIP_GAP4=rocn` | gap4_rootstuck_mem_20261109 | r・o・c・nは58問で揃った害がない; 600秒の最大RSS neos-957323 5597→3509等 (rocln) | 既定では使わない (rocとnを600秒47問で確かめる候補) |  |
| a | `MIP_GAP4=i` | gap4_rootstuck_mem_20261109 | neos-873061で根を縮めず解を悪くした | 効かない |  |
| a | `MIP_GAP4=q` | gap4_rootstuck_mem_20261109 | 58問×種1・2: SGM 16.07→15.75 (−2.0%、95%区間[−6.2%,+1.6%]、p=0.37)、解けた83→87 (noswot 2回・mc11・neos-960392)、揃った差なし; puriri 根の下界はHiGHSにほぼそろうが解が悪い (81380→160360、種2は解なし) | 既定の候補 (記録の判断)、puririでヒューリスティクスが解を見つけにくい | g4q |
| a | `MIP_GAP4=qk` | gap4_rootstuck_mem_20261109 | puriri 450秒×種1・2: やり直しのサブMIPもすぐ実行不能で解なし | 効かない | g4q |
| a | `MIP_TAB_HIGHS_RULES=1` | gap4_rootstuck_mem_20261109 | 58問×種1・2 (種2は56問): SGM 15.70→15.03 (−4.2%、95%区間[−10.3%,+2.3%]、p=0.20)、解けた87→87、メモリ−12%; neos-873061 根104.38→109.52 | 既定の候補 (記録の判断)、30n20b8のように密なtableauカットで下界を得る問題では下界が下がる | tab |
| a | `MIP_HO3_ENUM=1` | gap4_rootstuck_mem_20261109 | net12の根の下界の差 (前処理) がHiGHSの前処理の後のLPにほぼそろう | この記録では58問の計測なし |  |
| a | `MIP_TREE_NO_PASS_BASIS=1` | gap4_rootstuck_mem_20261109 | widden 最大RSS 4330→3167MB (rocl+) | 単独の判断なし (既存) |  |
| a | `MIP_GAP4=rocn` | gap4mem_20261111 | 58問60秒種1・2: SGM 17.66→17.68 (+0.1%、95%区間 −2.3%〜+2.3%)、解けた 82→84、最大RSS和 −8.1%。600秒: uccase12 RSS −24%、fiball −49%(557→445秒)。tbfp-network 2種とも時間切れ(既定も4回中2回) | 揃った害なし。tbfp-network の遅れを分けるまで保留(追記で1文字ずつでは害なし、揺れの範囲と判断)。取り込み基準は満たす |  |
| a | `MIP_GAP4=t` | gap4mem_20261111 | 58問: SGM +0.5% (−3.2%〜+4.6%)、解けた 82→83、RSS +0.9%。neos-873061 の600秒下界 既定104.72・104.42 対 104.42・104.09 | 新しい既定では効き目がない、候補から外す |  |
| a | `T_SPLIT_DEDICATED=1` | mf_regressions_and_fome13_hang_20261013 | LP、METIS 修正後 4スレッド・Mittelmann 10問・各2回: rayon 方式に対し幾何平均 1.002 倍、cygnet (4成分) で 1.19 倍遅い | 先に終わった成分のスレッドが残りを手伝えないので既定にしない (試験用) |  |
| a | `MIP_HO1_ROLEEARLY+MIP_HO1_SUBCUT` | highs_only_20261105 | neos-933966 が 600秒で 304.5 秒で解けた (1回、揺れを含む)。58問の数字なし | 1回だけ・揺れを含む。まとめでは採否なし (詳細は highs_only_20261105/G1.md) |  |
| a | `MIP_HO2_IMPLINT+MIP_HO2_GOBJ+MIP_TREE_RESTART` | highs_only_20261105 | csched008 の下界が 172 に (4回中2回)、解けない | 解けない、次の壁に当たる | TREEEFF |
| a | `MIP_TREE_RESTART` | highs_only_20261105 | swath3 が解ける (前の記録) | まとめでは採否なし | TREEEFF |
| a | `MIP_HO3_ENUM` | highs_only_20261105 | net12 の根の LP が HiGHS と同じ値に、解けない (ノードの LP が重い) | 解けない |  |
| a | `MIP_HO4_CUT0` | highs_only_20261105 | neos-873061 根の下界 95.9→110.4 (HiGHS 113.2)、mzzv11 で暫定解・下界・ノード改善、puriri は0ラウンド目が時間の上限に当たる。いずれも解けない | 解けない。根のカットのループの止め方が共通の手当て候補 |  |
| a | `MIP_HO5_CUTLP_GAIN` | highs_only_20261105 | widden 解けない | 解けない |  |
| a | `MIP_HO5_BGSTALL` | highs_only_20261105 | widden 解けない | 解けない |  |
| a | `MIP_TREE2_INTAGG` | highs_only_20261105 | widden が 359 秒で解ける (600秒、2回とも) | まとめでは採否なし (既存の試作) |  |
| a | `MIP_MC11FIX2=c` | mc11_wait2_20261103 | mc11 4回 SGM(+1) 15.5→16.3、根の下界 11666.4 (道Bに入らず 0/4) | 下界はほとんど上がらず、採らない |  |
| a | `MIP_MC11FIX2=ac` | mc11_wait2_20261103 | mc11 4回 SGM(+1) 15.5→17.7。6問: hypothyroid-k1 29.0→35.3、h80x6320d 0/2→46.9 | 採らない (重い分離で遅い) |  |
| a | `MIP_MC11FIX2=c+T_MIP_MC11FIX2_P2_ROUNDS=10` | mc11_wait2_20261103 | mc11 4回 SGM(+1) 15.5→19.4 | 採らない |  |
| a | `MIP_SYM_LEX_CANON` | highsonly16_summary_20261020 | neos-950242 解なし→4 (FJ_POOR_NOINC と併用時)。awhea・comp21-2idx 変わらず | 候補→後の記録で不採用 (組み合わせ C は neos-960392・h80x6320d を落とす) |  |
| a | `MIP_OBJ_SUPPORT` | highsonly16_summary_20261020 | app1-1 10.9〜14.0→6.3〜7.0秒 (7回とも根で)。40問で当たるのは graph20-20-1rand だけ | 不採用 (app1-1 は4コアでは既定でも解ける) |  |
| a | `MIP_FC_NODE_SEP2` | highsonly16_summary_20261020 | beasleyC3 時間切れ→19.6〜52.2秒 (5回中4回)、20問で解けた数 +1、h80x6320d +2〜5秒 | 不採用 (beasleyC3 を2回中1回) |  |
| a | `MIP_DEGEN_SCORE+MIP_DROP_FUTILE_CUTS` | highsonly16_summary_20261020 | 効果なし | 取り込まない | src に無い |
| a | `MIP_INEQ_SINGLETON=1` | highsonly16_summary_20261020 | physiciansched6-2 の根の LP を上げる。40問に当たらない | LP が重くなる恐れ、取り込むが候補にしない (後に不採用) |  |
| a | `MIP_LP_PROBE_AFTER_EXT=k` | lp_probe_after_ext_20261102 | ns1116954 FJ_EARLY=0 で 35.5・35.1・35.6秒 (base 31.7〜33.5)。根の LP neos-827175 748反復、neos-960392 2665反復 | 推し量りが外れて根の LP が長く残る。58問は測っていない |  |
| a | `MIP_LP_PROBE_AFTER_EXT=r` | lp_probe_after_ext_20261102 | 58問は計らず (固定が増えないので条件が起きない) | 実装と確認だけ |  |
| a | `MIP_LMIP3=sato` | lmip3_score_20261105 | 58問60秒1回: SGM 16.89→16.87、解けた 44→44。misc07 16.9→11.5・physiciansched6-2 25.6→16.6 速く、noswot 31.2→59.4 遅い。55問3秒単独: 良い24・悪い20 | 58問では揺れの範囲、既定は変えない。種を変えた複数回で見直す |  |
| a | `MIP_LMIP3=t` | lmip3_score_20261105 | 58問1回: SGM 16.89→17.24、解けた 44→43 (physiciansched6-2 25.6→60)。swath1 23.1→38.9 遅い、mc11・noswot 速い。55問: 良い20・悪い17 | 58問ではわずかに悪い、揺れの範囲 |  |
| a | `MIP_LMIP3=a` | lmip3_score_20261105 | 55問: 良い20・悪い21 | 効き目が見えない |  |
| a | `MIP_LMIP1_CAND=1` | lmip1_cand_20261105 | 58問60秒1回: 解けた 43→43、SGM 17.85→18.47 (+3.5%)、ギャップ和 2.282→2.410。noswot 19.7→56.2・misc07・drayage-25-23・qiu 遅い、eil33-2・mas76 速い。55問3種: 解なし 19→7 (h80x6320d・hypothyro | 既定にはしない。58問を種を変えて回してから決める (未実施) |  |
| a | `MIP_LMIP1_CAND=d` | lmip1_cand_20261105 | 55問3種: 解なし 19→22、良い56・悪い59・同じ50 | 効き目が見えない (切り分け用) |  |
| a | `MIP_LMIP2=l` | lmip2_bt_20261105 | 58問60秒 種0〜3の4回: SGM 4回とも −1〜−5% (種1〜3合計 21.71→21.10、−2.8%)、解けた 116→115 (174回)。noswot 56.6→33.6・misc07 30.8→19.1 速い、physiciansched6-2 32.9→48.7 遅い | 全体への効き目は小さいが害もはっきりしない。physiciansched6-2・30n20b8 を種を増やして見てから | lm2 |
| a | `MIP_LMIP2=b` | lmip2_bt_20261105 | 55問3秒単独: 良い18・悪い18 (2回目 17・18)。cvs16r128-89 で手の数 0.04倍 | 差し引きの効き目は見えず、目的の列が多い問題で1手が重い |  |
| a | `MIP_LMIP2=w` | lmip2_bt_20261105 | 55問: 良い17・悪い16 | 単独では効き目が見えない |  |
| a | `MIP_LMIP2=bl` | lmip2_bt_20261105 | 55問: 良い22・悪い16 (blw 22・17、bw 19・17) | b を入れた組は b なしより悪い |  |
| a | `MIP_SHIFT_HEURS=noint` | heur_shift_lns2_20261026 | 4回ずつ: 最適 154→153、SGM 22.45→22.80、主ギャップ平均 0.71→1.13 (neos-960392 4/4→0/4)。2回の計測では最適 36・36 で最良 | 合計で既定を上回らず主ギャップが悪い。前回の見込みは揺れ |  |
| a | `MIP_LB2=1+MIP_PROX2=1+MIP_HSUB_STALL=1` | heur_shift_lns2_20261026 | 56問2回: 最適 34・35→35・34、SGM 24.89・25.09→25.56・25.15、主ギャップ 1.224・0.807→0.808・1.576 | 揺れを越える差がない、停滞の判定が遅く呼ばれない |  |
| a | `TMP_MAX_UPD=300` | greenbea_20260921_084155 | greenbea: 正答 2.46秒 (base 2.73-2.80)、369us/反復 (-5%) | 経路を変える変更は誤答の危険 (fallback 未修正) | src に無い |
| a | `TMP_BUMP_FACTOR=16` | greenbea_20260921_084155 | greenbea: 正答 2.58秒、363us/反復 (-6%)。=8 は誤答 18.7秒 | 経路を変える、誤答の危険 (5-2 なしには適用不可) | src に無い |
| a | `MIP_INOUT_SEP=1+MIP_INOUT_EQ2_ONLY=1` | inout_sep_20261022 | 55問で当たるのは physiciansched6-2 だけ。下界 49324 は io 系9回中4回。単独では60秒で解けない | 使うならこの形だが、主側の手当てと組んではじめて意味がある | src に無い |
| a | `MIP_HIGHS_CUTMGMT=1` | neos-911970_20261020 | neos-911970 60秒: 55.4秒・時間切れ (54.7596)、打ち切り値あり 23.4秒 (既定 時間切れ 52.93) | 既存の選択肢の組合せとして調べただけ、採否なし |  |
| a | `MIP_CUTSEL_HIGHS=1+T_MIP_CUT_ROUNDS=100` | neos-911970_20261020 | neos-911970: 23.6・20.7秒 (既定 時間切れ)。打ち切り値ありでは 54.686 で止まる | 既存の選択肢の組合せとして調べただけ、採否なし |  |
| a | `MIP_TREE_CAP_WORST` | neos-911970_20261020 | neos-911970: 全体の下界 53.39 で止まる、暫定解 55.26 と悪い | 効かなかったので消した | src に無い |
| a | `MIP_GCDFORCE=1` | gcdforce_20261107 | 203問 (58問+Mittelmann) で当たる行が0本、求解は既定と同じ。58問の比べは回していない。初版は supportcase33 で行を誤って消し解が返らない回 (直した) | 何も変えないので既定にしない (消すか残すかはどちらでもよい) |  |
| a | `MIP_FACE_SEP_BG=off` | gfr_split_20261021 | mc11 2回とも解けない (11549〜11604 止まり)、BG だけで noswot 1回時間切れ | 効く場面がない (コードに残す、既定では使わない) |  |
| a | `MIP_RENS_DIVE+MIP_RENS_PROP+MIP_SUBMIP_RENS+MIP_SUBMIP_CUTS` | mzzv42z_neos-960392_20261020 | neos-960392 で効果なし (全て -226) | 効果なし (既存の試作を当てただけ) |  |
| a | `MIP_RENS_DIVE_RETRY=k` | mzzv42z_r3_20261020 | mzzv42z: 固定率 0.81 まで進むが近傍のサブ MIP は 0.11秒で実行不能、暫定解は RINS の -6620 止まり | 書いて捨てた | src に無い |
| a | `MIP_BG_CUTS_REHEUR=rins+MIP_RESTART_REF=1+MIP_RESTART_CUTS_READD=1` | mzzv42z_r4_20261021 | K 6回: mzzv42z -20340〜-20540、下界 -20745〜-20814 | GR (条件付き) にまとめた。READD は mik-250 の根を弱める |  |
| a | `MIP_MC11FIX1_RACE=s` | mc11_wait1_20261103 | 58問1回: 解けた 44→44、SGM(+10) 17.14→17.32 (+1.0%)。mc11 32.7→13.0 (4/4 で道B 13.8〜14.3秒)、noswot 29.9→16.6 速い、misc07 11.1→20.8・neos-911970 10.7→16.4 (4/4遅い)・qiu 23.3→59 | 58問では揺れの範囲。道が決まる利点はあるが全体を良くしない、bench に出さない |  |
| a | `T_MIP_CUT_ROUNDS=60` | mc11_r3_20261020 | mc11 60秒下界 11629.5・11634.0 (既定 11489〜11509)、暫定解 11708・11692。=100 で60秒以内に最適値を見つけた | 調べもの (根の時間が延び木に入るのが 15〜18秒)。FC_CUT_ROUNDS の形にした |  |
| a | `MIP_TREE_POOL_BB=1` | mc11_r3_20261020 | mc11 の既定で60秒の下界 +15〜+45、p5s・FC_CUT_ROUNDS とでは効かない | 全木の問題で動きが変わり副作用を確かめていない、既定にする候補ではない | src に無い |
| a | `MIP_MEM2=pb` | memcap_b_20261104 | 600秒: 最大 RSS 2.9〜4.4 GB、あまり効かない (b の上限に届かず捨てる処理が動かない) | 効かない |  |
| a | `MIP_MEMBQ_SUBRC` | membq_path_20261105 | n2seq36q 600秒: 2回とも道A 72・79秒 (rcq は 425〜518秒) | 速いのは rc と同じ道に当たるだけ。1問だけで既定にする根拠にはならない |  |
| a | `MIP_MEMBQ_LNSGROW` | membq_path_20261105 | n2seq36q 600秒: 道Bで rcq 429〜578秒→192.6・242.8秒 (3回中2回)、jitter と組では効かず 577.5秒 | 1〜2回ずつで揺れの範囲を含む。58問は測っていない |  |
| a | `MIP_MEMB=rc` | membq_path_20261105 | n2seq36q 600秒: rc 105〜108秒、rcq 425〜518秒 | 差は丸めの良し悪しでなく運の分かれ道。既定を q から外すかは多くの問題の平均で決めるべき |  |
| a | `MIP_INTVUB=2` | intvub_20261106 | mushroom-best の再スタートの模型の根は 0.01585 のまま | 効かない |  |
| a | `MIP_INTVUB=1+MIP_INTVUB_IMPL=1` | intvub_20261106 | mushroom-best の根は 0.01585 のまま | 効かない |  |
| a | `XO_MIP_CLEANUP_HINT=1+XO_IPM_RETRY_PC=3+XO_NO_BOUND_STOP_ZERO_COST=1+IPM_KKT_AUG_RATIO=4` | ipmx_harm_20261106 | ipmxm 58問60秒2回: SGM 14.11→14.21 (+0.7%)、解けた 89→88 (timtab1 の1回)。mc11 9.6・17.5・14.6秒 (既定 15.1・17.1・22.8)。大きい LP の対象8問では ipmx と同じ得 | 既定にしてよい候補だが60秒では得が見えない。Mittelmann 600秒で比べてから決める |  |
| a | `MIP_MODKVB=1+T_MIP_MODKVB_MAXCANDS=50` | modkvb_20261107 | mk50: 58問 −0.6% (解けた回 85→83)、Mittelmann 186問 +0.7% (区間 [−0.5%, +2.3%]、96→95)。600秒の効き目も消える | 害は揺れの範囲だが効き目も消える |  |
| a | `MIP_MODKVB=1+MIP_MODKVB_SKIPZH` | modkvb_20261107 | zerohalf と同じ候補を除いても根も60秒も良くならない (mk50 比 +2.0%) | 試作は zerohalf の代わりにならない |  |
| a | `MIP_MUSHAGG=1` | mushroom_cuts_20261106 | 根だけの比べ (再スタートの模型) | cover だけの形 (mc) を採った |  |
| a | `MIP_HNFCUT=1` | hnfcut_20261107 | abc: 58問 SGM 16.64→16.42 (−1.3%、区間 [−3.9%, +1.0%])、解けた 86→86。rococoB10 の b の効き目が消える (下界 11465・11335=既定と同じ) | a が b を打ち消す。11-08 に =1 を b に変えた |  |
| a | `MIP_HNFCUT=b` | hnfcut_20261107 | rococoB10 600秒下界 11340・11502→14591・14419、rococoC10 10536・10480→10701・10667 | 当たる問題が少ない (16問中3問) |  |
| a | `MIP_HNFPRE=1` | hnfpre_20261107 | 58問60秒 種1・2: SGM 15.39→15.24 (−0.9%、区間 [−4.1%, +2.8%])、解けた回 87→85。neos-952987 60秒で終わらない→0.14・0.19秒、enlight_hard 2.7→0.01秒 | 分枝の丸めだけの合同式は60秒で差が見えず後回し。bounds 相当を既定候補に |  |
| a | `MIP_HNFPRE_LAT=1` | hnfpre_20261107 | markshare_4_0 1.9・2.0→0.5・0.9秒、markshare_5_0 169・181→66・54秒。markshare1・2 は解を見つける力が落ちる回、pk1 は大きく悪い (条件で外した) | 試作のまま (解の質の落ちを防ぐ案) |  |
| a | `MIP_HNFSUBST=1` | hnfsubst_20261107 | 58問2回: SGM 20.46→20.23 (−1.1%)、解けた回 79→79 (置き換えは flugpl だけ)。traininstance6 600秒下界 876・1878→10293・5981。mzzv11 で根のカット0本の回 | 既定にする根拠は traininstance6 と flugpl の2問しかない |  |
| a | `MIP_HNFBR=1` | hnfsubst_20261107 | traininstance6 1200秒: 既定 640・971秒→468・622秒 (1.4・1.6倍速い) | 試作 (1問だけ) |  |
| a | `MIP_IMPLINT_TU2=bas` | implint_tu_20261110 | 刻みが付く5問: −8.5% (−30.9%, +6.5%)、pk1 23.3・17.6→12.6・12.6秒。csched008・tr12-30 は600秒で閉じない | s だけを既定の刻みの判定に足す形 (bns) を58問で確かめる案 |  |
| a | `MIP_ACS=1` | heur_sub_20261023 | 30n20b8 最初の暫定解 4回とも 25.5〜26.8秒 (既定 43.5〜54.6秒) だが最終は4回とも 553 (既定 504・504・453)。neos-1456979 では解が出ない | 最初の暫定解には効くが最終が悪い。ACS の後の木の扱いを見直してから | src に無い |
| a | `MIP_FARKAS_DIVE=1` | heur_sub_20261023 | 30n20b8・neos-1456979 で4回中0回 | 効かない、候補にしない | src に無い |
| a | `MIP_ZEROOBJ=1` | heur_sub_20261023 | 30n20b8・neos-1456979 で4回中0回 (1000ノード上限か時間切れ) | 効かない、候補にしない | src に無い |
| a | `MIP_RENS_HIGHS=1+MIP_DIVE_LOCK_ORDER=1+MIP_SUBMIP_HIGHS=-t` | highstree_phys_20261024 | 形A physiciansched6-2 60秒2回: 51029・49724 (近傍に 49324 を含むが解けない)。-t だけ 49724・49424 | 近傍を作るところまでは効くが60秒の結果は良くならない |  |
| a | `MIP_CUTSEL_PROBE=rate+T_MIP_CUTSEL_PROBE_ROUNDS=6+T_MIP_CUTSEL_PROBE_RATIO=3` | highstree_phys_20261024 | 計測18 (9問x4回): 最適 29 対 29。binkar10_1 平均 15→8秒、mc11 43→37秒、dcmulti 1→2〜3秒、roll3000 −1回。計測17 (RATIO なし) 23 対 23、計測16 22 対 21 | 基準 (既定を2回以上上回る) を満たさない。56問の計測に回す候補にはしない |  |
| a | `MIP_CUT_LP_HALVE+T_MIP_CUT_LP_CAP_MULT=3+MIP_CUT_CAP_FROM_LOOP` | halve_submip_20261030 | hm3_cfl 58問2回: SGM 18.59→18.40・19.33→19.13 (−1.0%・−1.0%)、解けた 42→43・41→41。dbl と重ねると +1.1%・−1.6% | 損なしの候補だが、揺れを超える差ではない。重ねると得が消えるので dbl を先に既定化 |  |
| a | `MIP_BG_CUTS_WAIT=1` | halve_submip_20261030 | bgw 58問 回2 +3.3%、回3 −6.9%、解けた数も回で入れ替わる。h80x6320d 49.9→59.3秒 | 揺れの範囲、どれも既定にしない (この時点) |  |
| a | `MIP_SUBMIP_HIGHS=-s+T_MIP_SUBMIP_RELIABLE=1` | halve_submip_20261030 | hs_r1 58問 回4・5: 解けた ±0・+2、mc11・noswot (回4) を落とす | 採らない |  |
| a | `MIP_DIVE_LOCK_ORDER=1` | halve_submip_20261030 | lock 58問2回: SGM 18.18→18.30 (+0.7%)、解けた 82→84。Mittelmann 45.15(47)→45.29(46)。neos-960392 0/3→1/3 | SGM が悪くなる、lock の順は既定にしない |  |
| a | `MIP_ROOT_BASIS_FIX=s` | halve_submip_20261030 | rbfs 58問2回: −1.1%、86→84。Mittelmann +0.7% (50→49)。neos859080 を 3.1秒で示す | 得も損も揺れの範囲、既定にする理由がない |  |
| a | `MIP_BG_CUTS_LP_CAP=1` | halve_submip_20261030 | bgcap 58問2回: 17.57→17.78 (+1.2%)、86→86。Mittelmann −0.6% | 得が見えない、採らない |  |
| a | `MIP_BG_CUTS_DSE=rw` | halve_submip_20261030 | dserw 58問2回: +0.9%、86→87。Mittelmann +0.4%。neos-960392 3回とも少し速い | 害も得も揺れの範囲、既定にする理由がない |  |
| a | `MIP_LOSE6_1_STOP=w+MIP_EARLY_RESTART_JOIN=1+MIP_LOSE6_1_LIGHT=0.2` | halve_submip_20261030 | l1 58問2回: −3.2% (85→87)、Mittelmann −0.1% (48→49)。h80x6320d 45.4・TL・TL→34.8・39.6・43.2。beasleyC3 12.3→30.2、piperout-27・misc07 遅い (1回ずつ) | 既定の候補 l1 (beasleyC3・irp を数回回してから)。後の bench_method で −3〜−4% は既定の運と判明 | ERWAIT |
| a | `l1+l2+l3 (LOSE6 全部)` | halve_submip_20261030 | l123 58問2回: −4.4% (85→86)、Mittelmann +0.1% (48→49)。irp 38.9、noswot 悪化、physiciansched6-2 で1回 TL | irp・noswot・physiciansched6-2 の TL がある |  |
| a | `MIP_ROBUST_LNSFREQ=1+MIP_ROBUST_IMPORT=1+MIP_MEMBQ_LNSGROW=1` | halve_submip_20261030 | lnsx 58問2回: −0.3% (86→86)、Mittelmann −0.3% (49→48)、片方以上解 −1.6% | 揺れの中 (60秒では働かない部品) |  |
| a | `MIP_S1_RINSWIDE=2+MIP_S1_IPMFIND=1+T_MIP_CLIQUE_FB_RATIO=0.1` | halve_submip_20261030 | s1 58問2回: −1.3% (88→88)、Mittelmann +0.2% (49→49) | 揺れ (±2.8%) の中、既定化に値する良さはない |  |
| a | `MIP_S3_TSDENSE=1` | halve_submip_20261030 | s3 58問2回: +0.3% (88→88)、Mittelmann +0.6% (49→48)。h80x6320d def だけ | 揺れの中 |  |
| a | `MIP_HO4_CUT0=1+MIP_TREE3_JOINCAP=1+MIP_S6_CUTLP_TCAP=10` | halve_submip_20261030 | s6 58問2回: +0.1% (88→89)、Mittelmann +0.3% (49→49)。neos859080 60秒超→36.6秒 | 揺れの中、1問だけ |  |
| a | `T_CUTSEL_FILL_FRAC=0.25` | mip_heur_cuts_impl_eval_20261012 | 根だけ21問: 根のギャップの和 4.276→4.305、30n20b8 143.6→126.5 | 根の下界がほとんど変わらない、求解全体は測っていない |  |
| a | `MIP_IMPLBD_CUTS` | mip_heur_cuts_impl_eval_20261012 | 根だけ21問: 根のギャップの和 4.276→4.274。graph20-20-1rand・neos-1456979 の下界は動かない | 採らない |  |
| a | `T_MIP_CLIQUE_MAX=20` | mip_heur_cuts_impl_eval_20261012 | 根だけ21問: 根のギャップの和 4.276→4.275、時間 191→200 | 根の下界はほとんど動かない、採らない |  |
| a | `MIP_CMIR_DELTA1` | mip_heur_cuts_impl_eval_20261012 | 根だけ21問: 根のギャップの和 4.276→4.272。supportcase26 1347.7→1353.0 | 採らない |  |
| a | `MIP_POOL_EVICT` | mip_heur_cuts_impl_eval_20261012 | 29問: SGM 30.52→30.32、主ギャップ (除く) 2.274→2.316、misc07 57秒→時間切れ | 差がない、採らない |  |
| a | `T_MIP_NOINC_RENS_NODES_MULT=4` | mip_heur_cuts_impl_eval_20261012 | neos-1456979: x1 2回中1回、x4 2回中1回、TRIES=2 3回中1回 | ノード数では直らない、採らない |  |
| a | `T_MIP_ROOT_HEUR_TIME_FRAC=0.15` | mip_heur_cuts_impl_eval_20261012 | 29問: SGM 30.52→29.91、主ギャップ (除く) 2.274→2.453 | ばらつきの範囲、採らない |  |
| a | `T_MIP_DIVE_MULTI_DIV` | mip_heur_cuts_impl_eval_20261012 | 25問: SGM 26.36→26.53。supportcase26 0.077→0.113、neos-860300 31.7→41.1秒、timtab1 良い | 採らない |  |
| a | `MIP_RESTART_NOINC_HEURS` | mip_heur_cuts_impl_eval_20261012 | 40問 (1スレッド) に暫定解なしで再スタートする問題がない | 効果を測れないので既定にしない |  |
| a | `MIP_POLL_SUBMIPS` | mip_primal_weakness_20261016 | 20問1回: 主ギャップ 4.888→5.200 | 不採用 |  |
| a | `MIP_FOCUS=primal` | mip_primal_weakness_20261016 | 20問1回: 主ギャップ 4.888→4.931 | 不採用 (既存) |  |
| a | `T_MIP_LNS_TIME_MIN=0.05` | mip_primal_weakness_20261016 | 20問1回: 主ギャップ 4.888→4.316。FJ2+FIX_CAP と組で 4.188 (組なし 4.067) | 単独では保留、組では不採用 |  |
| a | `KEEP_NOINC (FJ2 採用版と組)` | mip_primal_weakness_20261016 | 93問1回: 主ギャップ 14.63 (FJ2 採用版 13.77)、解けた 29、SGM 38.09。C で 6.39→6.89 | 見張りで良いが C で悪化、不採用 |  |
| a | `MIP_RINS_DIVE` | mip_primal_weakness_20261016 | 20問1回 (RENS_DIVE と組): 主ギャップ 3.544→3.105、解けた 2→1、SGM 56.33→57.96 | 改善と悪化が混ざる、不採用 |  |
| a | `MIP_PROOF_SWITCH` | mip_primal_weakness_20261016 | 証明18問2回: 8.5・37.62秒→9.0・36.44秒。93問2回: 28.5・37.66・12.99→28.5・37.67・13.09 | 揺れの範囲、不採用 (環境変数で残す) |  |
| a | `T_MIP_TREE_LOCAL_CAP=2` | mip_primal_weakness_20261016 | 40問: 24.5・20.58→25.0・21.40。h80x6320d 33→39〜41秒、eil33-2・rout・neos-860300 遅い | 不採用 (=8 も 24.5・21.00) |  |
| a | `MIP_CROSSOVER_FOCUS` | mip_primal_weakness_20261016 | 40問2回: 25.0・20.74・1.949→25.0・19.92・2.067。93問2回: 28.5・38.05・14.10→29.0・37.73・15.46。radiationm18-12-05 2回とも 0.83 | 主ギャップの和が1割悪い、不採用 |  |
| a | `T_MIP_CUT_FLAT_STOP=3` | neos960392_cutstop_20261101 | ほか 8 問で効いたのは neos-1456979 だけ (6 → 3 ラウンド、下界 154 同じ・結果同じ)。neos-960392 は (a) 全体で 0/5 | (a) は neos-960392 で 0/5、他は変化なし |  |
| a | `T_MIP_CUT_LP_WORK=0.1+MIP_CUT_LP_CAP_STOP=flat` | neos960392_cutstop_20261101 | neos-960392 4/5 (1 回は 2 ラウンド目前の時間判定で止まる)。physiciansched6-2 の 2 ラウンド目 LP (1166 反復) を外し根の下界 48684 に低下 | 時間の際が残る・physiciansched6-2 の下界低下 → LP_WORK_PREV を足した lpwp へ |  |
| a | `T_MIP_CUT_LP_WORK=0.1+MIP_CUT_LP_CAP_STOP=flat+MIP_CUT_CAP_FROM_LOOP=1` | neos960392_cutstop_20261101 | neos-960392 5/5 (38.5〜41.5 秒) | LP_WORK_PREV を足した lpwpc に置き換え |  |
| a | `T_MIP_CUT_LP_WORK=0.1+T_MIP_CUT_LP_WORK_PREV=5+MIP_CUT_LP_CAP_STOP=flat` | neos960392_cutstop_20261101 | neos-960392 5/5 (37.8〜40.0 秒)。後の bench12 (halve_submip_20261030.md、cs1): 58 問 2 回 SGM 18.44 → 18.46 (+0.1%)、解けた 83 → 82、Mittelmann 44.64 → 44.35 (−0.6%)、qiu を 2 | 2 ラウンド目前の時間判定の余裕が 0.1〜0.3 秒しかない。bench12 で qiu・physiciansched6-2・roll3000・neos-1171737 を失い「 |  |
| a | `MIP_SUBMIP_NESTED=i+T_MIP_SUBMIP_NESTED_TIME_MAX=0.5+T_MIP_SUBMIP_NESTED_NODES=50` | neos960392_nested_20261101 | 提案の形。後の bench12 (halve_submip_20261030.md): 58 問 2 回 SGM 18.44 → 18.68 (+1.3%)、解けた 83 → 84、Mittelmann 44.64 → 44.68 (+0.1%、解けた 46 → 44)。neos-960392 0/3 → 1/3。失っ | bench12 で「nesti は採らない」 |  |
| a | `MIP_CHILD_HIGHS=1` | node_lp_20261106 | 重い 3 問 300 秒 1 回: supportcase33・rococoC10 の反復変わらず、ns1830653 484 (既定 712) は揺れと区別できず | 子の向きは主因ではない (比べ) |  |
| a | `MIP_RESTORE_READD=2` | node_restore_20261021 | 共有だけ (P): mc11 下界 11599.0・11555.6、neos-2978193-inde 2 回とも時間切れ、neos-911970 41.5・38.7 | A3 との差が揺れか分からない | src に無い |
| a | `MIP_RESTORE_EXACT_SEL=144` | node_restore_20261021 | mc11 下界 11668・11614 (既定 11557・11591)、keep の反復 522〜564 → 296〜421 だが 1 回 132〜150 → 182〜185 ms。neos-911970 31.0 秒 (1 回) | 行の多い LP で BTRAN m 回の分 1 回の時間が増える。「既定にする候補ではない」 | src に無い |
| a | `MIP_RESTORE_COPY_DSE=128` | node_restore_20261021 | neos-2978193-inde (1 回) 時間切れ、poolfix 1397〜1616 反復で変わらず。keep (16) は既定と同じ | 効かない。採らない | src に無い |
| a | `MIP_RESTORE_NB=1` | node_restore_20261021 | 同じノードの比べで neos-2978193-inde の 1 ノードの反復 3〜5 割減。4 スレッドでは D 90.1 秒・時間切れ、A3+D 66.4・99.5 秒 (揺れの範囲)。mc11・neos-911970 変わらず | 4 スレッドの解く時間は揺れの範囲で決められない | src に無い |
| a | `MIP_NODECNT=p` | nodecnt_20261107 | kasai 1 スレッド 120 秒の下界 984495 → 986490 (h は 987472) | h の一部 (単独の 58 問なし) | TREEEFF |
| a | `MIP_NODECNT=1` | nodecnt_20261107 | kasai 1 スレッド 120 秒で固定 12791 本、下界・ノードは変わらないか重さで悪い | 効き目は見えない | TREEEFF |
| a | `MIP_NODECNT=hg` | nodecnt_20261107 | 種 1 60 秒: noswot 時間切れ → 27.4 (h 54.6)、qiu 23.1 → 32.9、binkar10_1 6.7 → 10.0、drayage-25-23 5.7 → 5.1 | 採られた割合で効く問題と効かない問題を分けられない | TREEEFF |
| a | `MIP_NODECNT=h+MIP_TREE1_POOL=1` | nodecnt_20261107 | kasai 4 スレッド 120 秒の下界 985701 → 987039 (1 スレッドの h は 987472) | kasai だけの確かめ | TREEEFF |
| a | `MIP_NODECUT_LIVE=1` | nodecutoff_20261106 | 58 問 60 秒 2 回: 解けた 44 → 43 / 43 → 44、SGM 11.48 → 11.62 / 11.27 → 11.22 (h80x6320d が入れ替わり、揺れ)。重い 4 問 120 秒 3 回で差なし。誤答 0 | 揺れを越える差なし。「LIVE は既定にしない」 |  |
| a | `MIP_NODELP2=pbx` | nodelp2_20261107 | 120 秒 種 1: rococoC10 は 4 スレッドとも厳密な重みを多く選び、traininstance6 は保存した重みを選ぶ | g なしの比べ (pbxg へ) |  |
| a | `MIP_NODELP_PERTURB=P+MIP_BASIS_NB=t` | nodelp2_20261107 | 120 秒 種 1: 1 回の反復 supportcase33 157〜180 → 86、ns1830653 520〜672 → 239、rococoC10 253〜294 → 176、ノード 1.3〜2 倍。traininstance6・tr12-30 ほぼ変わらず | 比べ (判定なしの摂動)。pbxg に組み込み |  |
| a | `MIP_TREE_SEP=1+T_MIP_TREE_SEP_HEAVY=1.25` | nodesep_aging_20261020 | 4 スレッド 60 秒: noswot 中央 42.1 秒 (既定 35.7、TREE_SEP 37.1)、beasleyC3 4/4 (既定 1/2)、neos-911970 2/4 (既定 0/2)、pk1 中央 25.8 (既定 22.9) | H だけでは noswot が良くならない |  |
| a | `MIP_TREE_SEP=1+T_MIP_TREE_SEP_HEAVY=1.25+T_MIP_TREE_SEP_CUT_AGE=30` | nodesep_aging_20261020 | 4 スレッド 60 秒 4 回: noswot 中央 38.9 (既定 35.7)、pk1 中央 22.9 (既定と同じ)、beasleyC3 4/4、neos-911970 2/4 (既定 0/2、前の TREE_SEP だけの 6/6 より弱い) | 「計測にかける組」として提案止まり (その後の採否は記録になし) |  |
| a | `MIP_OPTFACE_PARCUTS=3` | optface_cuts_20261105 | 58 問 60 秒 種 1・2: 解けた 39 → 42・39 → 40、SGM 15.01 → 14.46・13.55 → 13.83、主ギャップ 5.51% → 4.44%・4.82% → 4.81%。害は neos-911970 種 2 (20.8 → 時間切れ)、根の終わり中央値 4〜7% 延び。neos859 | (既定にする判断は書かれていない。ROBUST_ROOTIPM・ROOTLP_BOTH に及ばない、後の 3k 系も既定より良くない) | opf3 |
| a | `MIP_OPTFACE_PARCUTS=3s` | optface_cuts_20261105 | neos859080 中央値 17.4・時間切れ 2 回。beasleyC3 中央値 20.5・最悪 37.1・根の下界中央値 752.58 (一番良い) | neos859080 で渡さない形より悪い |  |
| a | `MIP_OPTFACE_PARCUTS=3sg+T_MIP_OPTFACE_SHARE_EPS=1e-4+T_MIP_OPTFACE_SHARE_START=3` | optface_cuts_20261105 | beasleyC3 中央値 16.2 → 16.2・最悪 32.3 → 29.1、根の下界中央値 749.9 → 751.4。58 問 種 1: 解けた 44 → 44、SGM 11.55 → 11.47 | 時間は既定と同じくらい、58 問で差はほぼない |  |
| a | `MIP_OPTFACE_PARCUTS=3k` | optface_cuts_20261105 | 58 問 60 秒 種 1・2・3: 解けた 129 → 129、SGM 10.82 → 10.89 (+0.4〜0.9%)。timtab1 3 回とも主ギャップ悪い (根の下界低い)。neos859080 中央値 3.3・最悪 13.6 (全設定で最速) | neos859080 型以外は既定と同じくらいで根の時間が数% 延びる分だけ損 |  |
| a | `MIP_OPTFACE_PARCUTS=3ky` | optface_cuts_20261105 | 58 問 種 1: 解けた 43 → 43、SGM 10.42 → 10.79。beasleyC3 で 3kx との差は揺れ | 58 問で既定より良くない |  |
| a | `MIP_OPTFACE_PARCUTS=3kz` | optface_cuts_20261105 | 58 問 種 1: 解けた 43 → 44、SGM 10.42 → 10.96 | 58 問で既定より良くない |  |
| a | `MIP_EQUIV_EXTEND_FIRST` | phys_equiv_20261025 | 延長の量は代入の後でも同じ (3644 行に 10857 個)、根は変わらない | 効き目なし |  |
| a | `T_MIP_CUTSEL_AGG_QUOTA=50` | physcmp_20261023 | 既定の予算で枠だけ: physiciansched6-2 は 2 ラウンドで止まり効かない | 枠だけでは効かない |  |
| a | `MIP_ENUM_PRESOLVE=1` | physcmp_20261023 | physiciansched6-2 では何も見つけない | 効き目なし (HiGHS の Enumeration は差の元ではない) |  |
| a | `MIP_FJ_LMIP_MIX=1+MIP_SYM_ORIG_CUT=1+MIP_CUT_AGG=1+MIP_RESTART_ALL_HEURS=1` | physcmp_20261023 | physiciansched6-2 2 回: 下界 48814・48894、暫定解 50826・49524 | 下界が届かず効かない |  |
| a | `MIP_FJ_LMIP_MIX=1+MIP_SYM_ORIG_CUT=1+MIP_CUT_AGG=1+T_MIP_CUT_EXTEND_MAX=0.5+MIP_RENS_DIVE=1` | physcmp_20261023 | physiciansched6-2 2 回: 下界 48814・48939、暫定解 50826・49624 | ダイビング式の RENS は下界が届かない回で判定できなかった |  |
| a | `MIP_RR_HIGHS=lock+T_MIP_RR_TRIES=20` | physhighs_20261024 | 書き出した近傍 r_0: カットの後の LP 解 (49424) から 20 回とも矛盾で失敗 (1 回 0.002 秒) | 丸めが当たらない |  |
| a | `MIP_RR_EACH_ROUND=1+MIP_RR_HIGHS=lock` | physhighs_20261024 | 近傍 4 つ 1 スレッド 30 秒: r_0 解なし→解なし、av_3 解なし→49324 (4.8 秒)、d1_3 49424 (3.8 秒)→49624 時間切れ、d2_3 49324 (6.8 秒)→解なし。各ラウンドの乱択丸めは 1 回も当たらない | 当たらない (解が出たのは Pump の当たり外れの入れ替わり) |  |
| a | `MIP_RENS_DIVE+MIP_SUBMIP_CUTS+MIP_SUBMIP_CUT_AGG` | physhighs_20261024 | 150 秒・AGG・再スタートなし: 固定率 0.60・自由な列 2737、サブ MIP 30 秒・17 ノードで解なし | 近傍は作れてもサブ MIP が解を出せない |  |
| a | `T_IPM_INIT_IPX=1` | physiciansched3_ipm_20261015 | 再生で 96 → 84 反復。尺度をそろえた後は 57 → 54〜59 で差なし | 全問題の初期点を変えるので既定にはしない (比較が要る) |  |
| a | `T_IPM_INFEAS_GUARD=1/10/100` | physiciansched3_ipm_20261015 | 再生 反復 101・95・97 (基準 96) | 発動しないか逆効果 |  |
| a | `T_NORMAL_DIAG_SCALE=1` | physiciansched3_ipm_20261015 | IPX の初期点と組んで 82 反復 (84)、分解の失敗は減らない | 分解の失敗は残る |  |
| a | `MIP_INEQ_SINGLETON` | physiciansched6-2_20261020 | 40 問に当たらない (1 万行以上の問題でしか動かない)。副作用は未確認 | MIP で双対の固定を使う影響が試験だけでは分からない |  |
| a | `MIP_CLIQUE_EXTEND_FIX` | physiciansched6-2_20261020 | (評価していない) | 未評価 |  |
| a | `MIP_DUALPROP` | physiciansched6-2_fj_20261020 | binkar10_1 で根の下界 6706.6 → 6718.0・ノード半分 (new の組)。physiciansched6-2 には効かない | 「候補 (優先は低め)」(その後の採否の記録なし) |  |
| a | `MIP_DOMCOL_PRED` | physiciansched6-2_fj_20261020 | fiber の 2 回目の DOMCOL で 8 → 14 本、時間 1.5・1.4 → 1.5・1.6 秒。physiciansched6-2 では 0 本 | 候補にしない (働く問題が少ない) |  |
| a | `MIP_SPARSIFY=highs` | physiciansched6-2_fj_20261020 | 最小の組には要らない (外しても FJ 16〜20 回) | 候補にしない |  |
| a | `MIP_INEQ_SINGLETON=1+MIP_DUAL_UB=1+MIP_CS_OBJ_PIVOT=1+MIP_FJ_LP_REPAIR=1+MIP_INC_LP_CAP=1+MIP_RINS_FJ_BOOST=1` | physiciansched6-2_r2_20261020 | P5: physiciansched6-2 49524・49324 (P3 51429)。uccase12 の 798739 は出なくなるが最適の証明 既定 2/3 → 0/5。副作用 60 秒: timtab1 平均 780.2k → 788.5k、eil33-2 37.6・39.1 → 44.9・36.8 | (記録では 40 問 + 12 問の対の計測の候補)。r3 の記録で p5s は親の対の計測で不採用 (beasleyC3・uccase12 で証明が間に合わない回、neos-14 |  |
| a | `MIP_INC_LP_CAP=1` | physiciansched6-2_r2_20261020 | P3 + CAP で uccase12 3 回とも 11507.41 (LP は 1.2 秒・993 反復で打ち切り)、ただし証明は時間切れ | (記録では「単独でも既定の候補」、既定の組で打ち切りが起きる問題は見ていない) |  |
| a | `MIP_RINS_FJ_BOOST=1` | physiciansched6-2_r2_20261020 | P3 + CAP + BOOST の 7 回: 49624 ×4・51429・49524・49324。r3: C1 + BOOST で 49724 ×2、p5s + PCR + BOOST 51629 ×2。FJ_P3=1 + BOOST 50826・50625 | 根で暫定解が FJ のものだけの問題で根が延びうる。単独の採否の記録なし |  |
| a | `MIP_FACE_SEP=off+T_MIP_CUT_TIME_FRAC=0.3+T_MIP_CUT_LP_TIME_FRAC=0.1` | physiciansched6-2_r2_20261020 | physiciansched6-2 根のカット 48587.4 → 48984.8 (10 ラウンド)、解には効かない | 候補にしない (面の分離は既定の理由があって入っている) |  |
| a | `MIP_CUT_EXTEND` | physiciansched_r3_20261020 | p5s + PCR + CUT_EXTEND: physiciansched6-2 下界 48838.3・48587.4、clique の fallback の 2 回目のループが 25 秒の回あり | 面の分離と LP の時間切れに阻まれて判定が働かない |  |
| a | `T_MIP_CUT_WIDE_MAX=200` | physiciansched_r3_20261020 | physiciansched6-2 (p5s + PCR) 下界 48760.1 (1 回) | 1 ラウンドの伸びが小さい (C1 より悪い) |  |
| a | `MIP_PROBE_EQUIV2=1+MIP_EQUIV2_CUTS=1` | physiciansched_r4_20261021 | physiciansched6-2 60 秒 2 回: 根の下界 48587 → 48940.29・48990.33、暫定解なし。副作用: hypothyroid-k1 3 回で 1 回時間切れ (揺れとみる)、nw04 14.8 → 13.1、neos-860300 51.2 → 45.4 | 「今のままでは候補にしない」(60 秒では解けず暫定解も出ない) |  |
| a | `MIP_PROBE_EQUIV2=1+MIP_EQUIV2_CUTS=1+MIP_EQUIV2_FJ=1+MIP_EQUIV2_TAB=1` | physiciansched_r5_20261021 | P5 (r5): physiciansched6-2 60 秒 暫定解 51029 → 50626、下界 48634〜48688 → 48957〜49215、解けない。副作用 5 問 2 回: 答え同じ、neos-960392 30.1・29.9 → 39.1・30.6 | 「既定の候補にしない (今のままでは)」働くのは physiciansched6-2 だけで解けた数・SGM は変わらない見込み |  |
| a | `MIP_EQUIV2_FJ=1 (E2C に足す)` | physiciansched_r5_20261021 | physiciansched6-2: 代入後の問題でも FJ が 21 回すべて解 (19 回は 50626、3.2〜4.2 秒) | PROBE_EQUIV2 とセットでしか意味がない | src に無い |
| a | `MIP_SUBMIP_CUT_AGG=1` | physprimal_20261024 | 書き出したサブ MIP: d2_3 が 5.7 秒・0 ノードで 49324 (最適)、d1_3 3.5 秒で 49424。実際の 60 秒の回では根の RINS は 0 ノードで時間切れのまま | 「どれも候補にしない」(60 秒の証明 0 回) |  |
| a | `MIP_SUBMIP_RENS_CUTOFF=1` | physprimal_20261024 | (記録内に比べの数字なし。RENS の近傍に解がないことは HiGHS で確かめた) | 候補にしない |  |
| a | `MIP_POOL2_TAKE=h` | pool_bound2_20261104 | 58 問 60 秒 1 回: 解けた 44 → 43 (qiu を落とす)、SGM 18.00 → 18.03。supportcase18・fiball で最大 RSS 11.6 → 6.9 GB・11.2 → 6.3 GB | 「既定は変えない」(振れの範囲) |  |
| a | `MIP_POOL2_TAKE=g,i,k4` | pool_bound2_20261104 | 58 問 60 秒 1 回: 解けた 44 → 44、SGM 18.00 → 17.54 (−2.6%)。600 秒 kasai 下界 984411.6 → 985386.9 (1 回)。速い swath1 49.6 → 25.6・noswot 57.8 → 22.8、遅い misc07 9.7 → 19.0 | 1 回だけで振れの範囲。既定を変える根拠にならない | pool2 |
| a | `MIP_POOL_DSE=1 (単独)` | pool_dse_20261102 | 8 問: SGM 33.67 → 36.07、neos-911970 14.0 → 37.4 (探索の順の振れ) | 単独は効かない (受け取った基底がほぼ全件特異で重みが使われない) |  |
| a | `MIP_PREPAR=ple` | prepar_20261107 | 58 問 60 秒 種 1・2: SGM +1.1% (95% 区間 −1.4%〜+4.2%)、解けた 81 → 81、片方だけ解けた回なし、初めの解は有意に早い。重い 55 問で前処理 1186.6 → 1065.4 秒 (−10%)、600 秒の 8 問 x 2 回で SGM 67.0 → 58.7。前処理の結果は | 既定にはしていない (前処理は全体の約 2% で効き目は前処理の重い問題に限られる) | prepar |
| a | `MIP_PRESOLVE_GI=1` | presolve_gap_20261022 | 一般整数のある 14 問 2 回: neos-950242 12.9・12.8 → 7.5・7.8、rout 12.7・10.5 → 7.5・7.7、mik-250 6.0・6.0 → 4.4・5.1、mzzv42z 最適値 3 回中 1 回 (既定 0/2)、悪くなった問題なし | (記録では「候補」、親の 40 + 16 問の対の計測で決める)。その後の採否の記録は見当たらない | src に無い |
| a | `MIP_DUAL_BOUND2=1` | presolve_size_20261025 | gt2 の LP 13460.2 → 20146.8 (HiGHS と同じ)、flugpl 15 → 14 行。55 問で形が変わるのは 2 問だけ。nw04 (DUAL_BOUND2 だけ) 9.7・11.5 秒 (既定 10.1〜10.5) | (記録では改善策の 1 番「すぐ 40 + 16 問で測る候補」)。その後の採否の記録は見当たらない |  |
| a | `MIP_PROBE_BASIS_GUARD=x` | probe_basis_20261106 | 58 問 60 秒 1 回: 一度も切り替わらず (根の LP は既定と同じ)、解けた 44 → 44 (幾何平均 9.93 → 10.96 は機械の再起動を挟む)。300 秒: momentum1 37.8 秒・96253.6、rail01 67.2 秒 | (記録の勧め「既定にするなら x」)。後の改良で g を候補にした |  |
| a | `MIP_PROBE_BASIS_GUARD=s+MIP_HO4_CUT0=1` | probe_basis_20261106 | rail01 300 秒: 下界 -90.94 → -89.77 (既定並みに戻る) | rail01 だけの確かめ |  |
| a | `MIP_CUT_HARRIS2=1` | puriri_cutlp_20261106 | puriri 600 秒: 根の下界は既定と同じ (カットの LP は進みの判定で先に止められる) | 単独では根の下界が変わらない |  |
| a | `MIP_RCSTRONG=ir` | rcstrong_20261111 | 58 問 60 秒 種 1・2: SGM 17.16 → 17.09 (−0.4%、95% 区間 −3.6%〜+2.3%)、解けた 83 → 84。当たる 13 問だけで −0.9% (区間 −6.5%〜+4.5%)。pk1 8 種で揺れの内。誤答 0 | (取り込みの基準を満たす、効き目は揺れの内で小さい) | cand1112 |
| a | `MIP_RCSTRONG=k` | rcstrong_20261111 | 58 問 60 秒 種 1・2: SGM 17.82 → 17.45 (−2.1%、95% 区間 −4.9%〜+0.2%)、解けた 83 → 81 (mc11 種 1・2、noswot 種 2、揺れと見る。4 種で既定 3 回・k 2 回落とす)。physiciansched6-2 は k だけ 1 回解く | (取り込みの基準を満たす、効き目は揺れの内) | cand1112 |
| a | `MIP_REDSPLIT=b+T_MIP_REDSPLIT_TFRAC=0.05` | redsplit_20261107 | 58 問 60 秒 種 1・2: SGM 15.15 → 15.02 (−0.8%、95% 区間 −6.4%〜+4.9%)、揺れの大きい 12 問を除き +0.3% (区間 −4.0%〜+5.0%)、解けた 86 → 87、根の下界のずれ −0.138 (p=0.001)。600 秒 timtab1・30n20b8・g | 「既定にはしない」58 問で害はないが速くもなく、効く問題が少ない。Mittelmann で見てから | rs |
| a | `MIP_REDSPLIT=a` | redsplit_20261107 | 58 問 60 秒 種 1・2: SGM 16.59 → 16.59 (+0.0%)、揺れを除き −1.0%、解けた 84 → 85、根の下界のずれ −0.011 | k-cuts は効かない (既存の CMIR がもともと k = 1,2,4,8 の k-cuts) |  |
| a | `MIP_RENS_MORE_NODES_REAL=1+MIP_RENS_MORE_NODES_SUB=0` | rens_more_real_20261101 | 深さ 2 のやり直し 0 本。4 問 × 3 回: n5-3 45.4・neos-2978193-inde 40.6 秒 (real 43.3・37.0)、揺れの中 | 深さ 2 の扱いは 58 問の結果が要る。まず real を確かめる |  |
| a | `MIP_RENS_MORE_NODES_REAL=1+T_MIP_RENS_MORE_NODES_SUB_MULT=2` | rens_more_real_20261101 | 深さ 2 のやり直しの秒 約 3 割減 (18.4 → 12.6)、親を良くした本数は同じ 5。4 問 × 3 回で揺れの中 | まず real を確かめる |  |
| a | `MIP_RESTART_CUTS_FIX=1` | restart_cuts_fix_20261028 | (c) 修正 1・2・3: exp-1-500-5-5 中央値 8.56、p200x1188c 3.93 (4 スレッド)。16 スレッドでは p200x1188c 3.4 → 4.8 秒と遅い。カットを問題の行にすると前処理が効かない (947 行 vs 266 行) | 修正 3 は SUBST で要らなくなる |  |
| a | `MIP_RESTART_CUTS_FIX=1+T_MIP_RESTART_CUTS_ROWS_FRAC=2+MIP_RESTART_CUTS_SUBST=1+MIP_RESTART_HCM_KEEP=1` | restart_cuts_fix_20261028 | (e): exp-1-500-5-5 8.79 → 8.28 (変わらず)、p200x1188c 3.60 → 4.51 秒 (ノード 1 → 23) | 候補にしない |  |
| a | `MIP_SUBMIP_HIGHS=-s+MIP_SUBMIP_PC_INHERIT=1+T_MIP_SUBMIP_RELIABLE=0+MIP_BG_CUTS_WAIT=1+MIP_SUBMIP_HIGHS_RESTAR` | restart_letters_20261031 | P_ch: 9 問 SGM 11.6 (P_r と同じ)、損の 4 問 8.1、physiciansched6-2 5/8。h80x6320d の 7 回 (P_ch・P_cht) で誤った最適値 6386.09905 (正 6382.09905、前処理の強制行の判定の誤り、後に修正) | TRIM=r で足りる、swath1 では P_r が一番速い |  |
| a | `MIP_RESTART_PC=1` | restart_pc_20261031 | 9 問 × 3 回: 再スタート 110 回すべてで pseudocost は空 (観測 0)、hs_pi0_bgw と同じ動き (binkar10_1 9.7〜10.4、swath1 33〜41) | 効かない (根での再スタートでは写すものがない) |  |
| a | `MIP_RESTART_FIRST_LPW` | restart_thr_20261031 | 働くのは physiciansched6-2 だけで、解けた回は変わらない (1/8) | 採らない (FIRST_FAC=0.01 で足りる) |  |
| a | `T_MIP_RESTART_FIRST_FAC=0.005` | restart_thr_20261031 | 0.01 と分かれるのは固定 0.5〜1% の問題 (58 問では blp-ar98 の 1 回、どちらも時間切れ) | 前の記録と揃えて 0.01 にした | RSTCRIT |
| a | `MIP_RESTART_ROOT_CUTS=1` | restart_root_20261104 | 58 問 60 秒 1 回 (rr、根の再スタートでは p だけ働く): 解けた 43 → 44、SGM 16.43 → 16.78。h80x6320d 時間切れ → 57.9、misc07 17.5 → 8.0、qiu 25.0 → 45.7。再スタートした 23 問中 14 問で根の値が全く同じ | 効き目も害も揺れの範囲。既定は変えない |  |
| a | `MIP_RESTART_ROOT_CUTS=f` | restart_root_20261104 | kasai・swath3 の 1 スレッドの木の中の再スタートで締まる列 0 本 | 効かない (4 スレッドでは使えない) |  |
| a | `MIP_ROOT_SUBMIP_HL=1` | rins_fix_20261022 | physiciansched6-2 60 秒: 暫定解 既定 51029 → 49324〜49424 (5 回中 4 回)、e2all 50626 → 49324〜49424 (5 回中 2 回)、最適値 10 回中 2 回、下界は届かず解けない。条件つきの形ではほかの問題で働かない見込み (timtab1・uccas | 「今のままでは既定の候補にしない」(解けた数・SGM は変わらない見込み) | src に無い |
| a | `MIP_SUBMIP_CUTS=1+MIP_RINS_FJ_BOOST=1` | rins_fix_20261022 | e2all の上で physiciansched6-2 50626 → 49824 (1 回)。SUBMIP_CUTS だけ・RINS_FJ_BOOST だけ・RINS_PROP・RINS_DIVE は 50626 のまま | 1 回だけ、既存の試作の組み合わせ |  |
| a | `MIP_BG_DETERM_ROBUST=p` | robust_stall_20261102 | 当たりつけ1回: roll3000・g で 27 ラウンド・12483.2 で止まり時間切れ (apo は 49.6秒) | HiGHS のスコアの道は roll3000 では第1段階より弱い。本番に出していない |  |
| a | `MIP_BG_DETERM_ROBUST=d` | robust_stall_20261102 | mc11・hypothyroid-k1 のどちらでも当たらない (暫定解との差が始めの 37%・78% 残る、閾値 5%) | 働かない |  |
| a | `MIP_BG_DETERM_ROBUST=w` | robust_stall_20261102 | hypothyroid-k1 で効かない (主のループが長く上限が安全網より大きい) | 効かない。時間で決まるので運を除く狙いと逆向き (費用を見るためだけ) |  |
| a | `MIP_FACE_RESCUE=1` | roll3000_r3_20261020 | roll3000 根が止まる回 10/25→0/14 (根 12211〜12368) だが 60秒では解けず、120秒は 3/3 (既定 2/3) で速くはない。supportcase26・neos-1456979 の下界は上がるが neos-1456979 で解が出る回 1/2→0/2。40問中働くのは5問 | roll3000 を 60 秒で解けるようにしない。neos-1456979 で解が出なくなる回があるので 40問・16問の対の計測で確かめるまでは既定にしない | src に無い |
| a | `T_MIP_RESTART_FIRST_FAC=0.01` | roll3000_r3_20261020 | roll3000 60秒で 3/9 (既定 1/4)、120秒では差なし。40問では p0201 だけ働く (1.12・1.30→0.86・1.05秒) | roll3000 では 120 秒で差が見えないので候補にしない | RSTCRIT |
| a | `MIP_NO_RESTART=1` | roll3000_r3_20261020 | roll3000 60秒で 3/9 (既定 1/4)、120秒では差なし | うまくいかなかった試み |  |
| a | `T_MIP_CUT_ROUNDS=50` | roll3000_r3_20261020 | roll3000 4回とも時間切れ (上界 12890〜13156)。止まらない回は根 12405 まで上がるが止まる回は 12146 のまま | うまくいかなかった試み |  |
| a | `MIP_FACE_RESCUE=1+MIP_CUT_ADAPT_ROUNDS=1` | roll3000_r3_20261020 | roll3000 4回: 根のカットの時間の上限 (6秒) で止まり延長は一度も起きず | 効果なし |  |
| a | `MIP_ROOTLP3=s` | rootlp3_20261111 | 58問×種1〜2: SGM 16.87→17.01 (+0.8%、95%区間 [−3.1%, +5.5%]、p=0.74)、解けた 83→84。揃った差 10teams (+0.59×2、試作は何もしていない揺れ)。誤答 0、RSS 比 1.010 | 既定候補にならない。害もほぼないが効きがない (特異な基底は小さい LP がほとんどで単体法が勝つ) |  |
| a | `MIP_ROOTLP3=3` | rootlp3_20261111 | 58問×種1〜2: 3本同時が当たったのは 3問6回 (hypothyroid-k1・30n20b8・blp-ar98)。3時間前の既定と比べ SGM −9.5% だが当たらない55問でも −10.5% で時期の差。誤答 0、RSS 比 1.004 | 58問では伸びしろがほぼない (重い根の LP がない)。同時期の対の計測を依頼中 (結果は記録の後半/未確認) |  |
| a | `MIP_ROOTLP_BOTH=1` | rootlp_both_20261105 | neos859080 (10回) 中央値 188.9→6.6秒・最悪 300→13.6秒。58問 種1: 解けた 40→41・SGM 14.18→14.05、種2: 38→37・14.76→14.92 (差ははっきりしない)。根の終わりの時刻 中央値 1.03〜1.06倍・幾何平均 1.09〜1.10倍 | 58問では差がはっきりしない。負けた側が裏で CPU を使い根の時間が 5〜10% 延びる害。次は安く篩う・内点法が負けた側では待たない |  |
| a | `XO_MIP_CLEANUP_HINT=1` | rootlp_ipm_fail_20261105 | MIP の中では CLEANUP_HINT=3 と同じ働き (58問は CLEANUP_HINT=3 で取った) | 上と同じ (揺れの範囲)。既定にするならこちら |  |
| a | `XO_DUAL_PUSH_FAST=1` | rootlp_ipm_fail_20261105 | opm2-z10-s4 内点法だけ 126.2→86.4秒、競争の形 123.9→94.4秒 (まだ二段解法 82.6秒に負ける)。他の10問は変わらない | 既定では使わない (記録に既定化の判断なし) |  |
| a | `XO_MIP_SCALE=1+T_XO_UNCONVERGED_GIVEUP=0+T_IPM_NOIMPROVE=10 (試作一式5: 上の4つ+DUAL_MEGIDDO_RACE_NOLIMIT=1+MIP_ADOPT` | rootlp_ipm_fail_20261105 | 11問すべて最適頂点 (momentum1 頂点なし→13.5秒)。58問1回: 負けている21問 15.02 (19/21)→12.86 (20/21)、MIPLIB40問 18.51 (26/40)→18.04 (27/40)。遅い側 10teams 5.4→29.1 (3回中2回遅い)、hypothyroid-k | 58問の多くは根の LP が小さく差の多くは揺れ。害の目立つものはない (既定化の判断はこの記録ではされていない) |  |
| a | `SIMPLEX_GROW_TA=1` | rootlp_simplex_fail_20261105 | supportcase42 を解けない。ほかの問題には回していない | 効かなかった |  |
| a | `SIMPLEX_HARRIS_WIDE=1` | rootlp_simplex_fail_20261105 | supportcase42 を解けない。ほかの問題には回していない | 効かなかった |  |
| a | `SIMPLEX_TINY_SKIP=1` | rootlp_simplex_fail_20261105 | supportcase42 を解けない。ほかの問題には回していない | 効かなかった |  |
| a | `T_XB_CONSISTENCY_TOL=0` | rootlp_simplex_fail_20261105 | supportcase42 を解けない。HARRIS2=t と合わせると仕上げ 9.4万反復で時間切れ | 効かなかった |  |
| a | `SIMPLEX_HARRIS2=t` | rootlp_simplex_fail_20261105 | supportcase42 は r と同じ経路 31.4秒。+XB_CONSISTENCY_TOL=0 で時間切れ。案A (+XB_TOL=0+NOISE_RESET=0.1) は最適 22.7秒だが uccase9 の経路を変える | B を推奨 (A は他の問題の経路を変える) |  |
| a | `SIMPLEX_SCALE=1+T_SIMPLEX_SCALE_MIN_M=10000+T_SIMPLEX_SCALE_MIN_RANGE=1e4+SIMPLEX_HARRIS2=r+SIMPLEX_NOISE_RESE` | rootlp_simplex_fail_20261105 | Mittelmann 29問 二段解法が最適で終わる 25→28問 (supportcase42・uccase12・neos-873061 が新たに解ける)。58問1回: 負けている21問 14.26 (20/21)→15.30 (19/21)、MIPLIB40問 18.59 (27/40)→18.86 (26/40) | 既定にする候補。既定にするなら 58問を 3 回ほど回して揺れを均したい (この記録の中では既定化されていない) |  |
| a | `MIP_ROOTSTOP=b` | rootstop_20261111 | 58問×種2: SGM 15.79→15.74 (−0.3%)、95%区間 [−3.5%, +2.9%]、解けた 87→87、揃った害なし。p200x1188c 4.3・7.8→2.5・3.4秒 | 取り込みの基準を満たすが効き目は小さい (この記録の中では既定化されていない) | cand1112 |
| a | `MIP_SYMPRE=probe` | sassy_20261110 | 58問×種1・2: SGM 15.91→15.84 (−0.5%)、95%区間 [−5.3%, +4.5%]、p=0.86、解けた 86→85、食い違い 0/116。10teams +0.50 log (対称性がなく探りが動かない揺れ)。neos-827175 生成元 66→121 等 | MIP の時間での効き目が見えない (害も見えない)。既定にはしない | cand1112 |
| a | `MIP_SYMPRE=1` | sassy_20261110 | 6問×種2: 既定とほぼ同じ (neos-827175 9.5/6.9秒 等)。58問は測っていない | 効き目が見えない。次数0・1の除去は勧めない | cand1112 |
| a | `MIP_FACE_ADAPT` | sepspeed_20261023 | 13問×2回: 10teams 時間切れ・11.5→13.3・4.8秒、physiciansched6-2 1ラウンド 3.1→1.2秒。neos-911970 21〜24→28秒 (根は同じ) | 記録では候補 (40問で確かめてから)。後の 40+16問 2組 (sq=SEP_FAST+FACE_ADAPT) で解けた数が合わせて1減り保留 | face |
| a | `MIP_SEP_FAST` | sepspeed_20261023 | roll3000 の CMIR 30→22 ms。カットは同じだが丸めの差で経過は変わる | 効果が小さいので急がない |  |
| a | `MIP_SEP_FAST+MIP_FACE_ADAPT (Q)` | sepspeed_20261023 | 13問×2回: 解けた回数 既定14→15 (10teams +1)。neos-911970 28〜28秒 (既定 21〜24)。後の 40+16問 2組: 40問 26・20.72→26・20.99、25・19.76 対 26・19.94 (noswot 落とす)、16問 neos-960392 で入れ替わり | 40問で解けた数が合わせて1減るので保留 | face |
| a | `MIP_GEN_FRACSKIP` | sepspeed_20261023 | roll3000・mzzv42z で呼び出しがほとんど減らない | 効果なし |  |
| a | `MIP_TAB_BUDGET` | sepspeed_20261023 | 最初のループ (4〜25 ラウンド) では予算に届かない | 効果なし |  |
| a | `SIMPLEX_WARM_SING_REPAIR=3` | singular_fix_a_20261102 | 10問×3回 −12.4%。58問1回: SGM 17.87→17.67 (−1.1%)、解けた 43→42 (neos5)。noswot 33.3→18.5・roll3000 50.7→32.4 速く、pk1 21.4→37.5・misc07 7.1→14.8・h80x6320d 42.2→57.5 遅く (1回)。 | 58問では揺れの範囲を出ず、pk1・misc07 のように遅くなる問題もある。壊れた基底は直しても crash より遠いことが多い |  |
| a | `SIMPLEX_WARM_SING_REPAIR=1` | singular_fix_a_20261102 | 同じ LP で反復が減る問題 (neos-911970 −40%、noswot −25%) と増える問題 (swath1 4倍、neos5 1.3倍、前処理の基底からの根 1.1〜8倍)。Netlib 15問は目的値一致 | 既定にはしない (=3 と同じ判断) |  |
| a | `MIP_ROOT_BASIS_FIX=s (rbfs)` | singular_fix_c_20261102 | bench15: 58問2回 18.42→18.23 (−1.1%、86→84)、Mittelmann +0.7% (50→49)。neos859080 を 3.1秒で示した | 得も損も揺れの範囲で既定にする理由がない |  |
| a | `MIP_EARLY_RESTART_SKIP_LIGHT=1` | skiplight_20261029 | 58問 (16スレッド・60秒・2回): 21問 21.00/21.09→21.07/21.14、40問 18.08/18.24→18.29/18.36、解けた数は減る。mc11 58.1→40.4・noswot 19.6→14.6 良く、neos-960392 45.7→時間切れ・timtab1 49.9→時間切れ | SGM は既定と同じか少し遅く、解けた数は減る。既定にしない |  |
| a | `T_MIP_EARLY_RESTART_FRAC=0.05` | skiplight_20261029 | 21問 21.15/21.17、40問 18.13/18.58 (既定 21.00/21.09・18.08/18.24)、解けた数は減る | 既定にしない |  |
| a | `MIP_RENS_MAX_DEPTH=1` | small_overhead1_20261102 | 担当5問×3回: dcmulti 2.24→1.77秒、ほかは揺れ | 58問で確かめる価値がある (未測定) |  |
| a | `MIP_SUBMIP_CUT_AGG=1` | submip_20261024 | physiciansched6-2 の RINS のサブ MIP 3つで解けたのは 0 (手当てなしと同じ) | 効かない |  |
| a | `MIP_SUBCAP=JT` | submip_cap_20261106 | 58問×種1・2: SGM 16.77→16.54 (−1.4%)、95%区間 [−3.8%, +1.2%]、解けた 84→83、時間切れの最終ギャップ +0.024 (p=0.02、neos-1456979)。trento1 根 297〜314→55〜90秒、thor50dday 84378→60240 (600秒種 | 得は 300〜600 秒の2問だけで、遅れて良い解を出す重いサブ MIP を切る害 (neos-873061 種2) が残る。まだ既定にしない | subcap |
| a | `MIP_SUBMIP_HL2=1` | submip_hl_20261022 | 60秒×4回: physiciansched6-2 既定 50826〜51029→49324〜49624 が 3/4、neos-960392 最適 2/2、mzzv42z 平均 −19542→−20085、働かない問題 (roll3000・beasleyC3・nw04) は揺れ | まだ候補にしない (2回ずつで揺れが大きい、条件に当たる問題数を見ていない)。親の全問計測に入れる価値はある | src に無い |
| a | `T_MIP_SUBMIP_OUTCOME_MIN_NODES=20 (om20)` | submip_nodes_20261101 | 9問×3回: 損の3問を戻さない (n5-3 51.1、neos-1582420 35.9) | 効かない |  |
| a | `T_MIP_SUBMIP_NODES_MULT_DEEP=0.2 (deep02)` | submip_nodes_20261101 | 9問×3回: 損の3問を戻さない | 効かない |  |
| a | `T_MIP_SUBMIP_NODES_MULT_TREE=0.2 (tree02)` | submip_nodes_20261101 | 9問×3回: 損の3問を戻さない | 効かない |  |
| a | `MIP_SYMDYN=1` | symdyn_20261110 | 対称性のある20問 (58問の内) で dyn +1.4%、区間は0をまたぐ。neos5・qiu 速く、assign1-5-8 ギャップわずかに悪い | 生成元ごとの比べだけでは推移的な群で orbital fixing より弱い (cod105) |  |
| a | `MIP_SYMSCALE=1` | symsign_20261106 | 58問・Mittelmann で何も変えない (both で +1.3% は揺れ) | 効かない |  |
| a | `MIP_TREE1_POOL=1 (tp)` | tree_close1_20261104 | kasai 4スレッド300秒: 下界 983855〜984381→985429。58問単独は未測定 | (trp と一緒に) 既定は変えない。TREE1_POOL と分けて測り直す必要 | TREEEFF |
| a | `MIP_CLIQUE_CUTS=on` | tree_close2_20261104 | rococoC10-001000 (FORCEQ+INTAGG と) 根 9870→10304、600秒 下界 10928→11038 | 担当1問だけ。58問未測定 | CLIQFB |
| a | `MIP_TREE3_CLIQUECAP=1` | tree_close3_20261104 | puriri の clique の LP 285→25秒。60秒では一度も働かない | 600秒向け。担当の問題で新しく解けた問題なし |  |
| a | `MIP_TREE3_BGCAP=1` | tree_close3_20261104 | 60秒では一度も働かない | 600秒向け |  |
| a | `MIP_TREE3_DIVECAP=1` | tree_close3_20261104 | 60秒では roll3000・neos-1456979 だけ働く。puriri で CLIQUECAP と合わせ初めて暫定解 81460 | 600秒向け |  |
| a | `MIP_TREE4_RESTART=1 (t4r)` | tree_close4_20261104 | sct2 600秒 下界 −231.0280→−231.0134 (閉じない)。58問1回: 16.60→16.62秒、44→44、試作の再スタートは起きていない | 担当の問題を閉じない。既定にしない |  |
| a | `MIP_TREE4_RCUNION=1` | tree_close4_20261104 | sct2: 固定の数は既定と同じ (62・338列) | 効かない |  |
| a | `MIP_TREE4_PROOFCUT=1` | tree_close4_20261104 | sct2 300秒で 17 ラウンド・19行・刈り込み1 | 効かない |  |
| a | `MIP_TREE_RESTART=1+T_MIP_TREE_RESTART_MAX_NODES=1e8 (trst)` | tree_close4_20261104 | sct2: 277秒に再スタート、下界 −231.0263、暫定解 −230.9695 (悪い) | 閉じない | TREEEFF |
| a | `MIP_TREE_RESTART=force+MIP_TREE_RESTART_GUARD=kc` | tree_restart_guard_20261106 | 58問×2回: SGM 16.92→16.70 (−1.3%、84→85)。neos5 28.8→45.0、mas76・misc07 遅い回、mc11 速い。swath3 種1 73秒 (def 215)、pg5_34 479秒 (def TL) | 58問では既定に入れるほどの差が出ない | TREEEFF |
| a | `MIP_TREE_RESTART=inc+MIP_TREE_RESTART_GUARD=c` | tree_restart_guard_20261106 | 58問×2回: 16.92→16.99 (+0.4%、84→84)。binkar10_1 2回とも約+4秒 (6.0→10.1、5.9→9.9) | 揺れの中。c は mc11 の良い再スタートも止める | TREEEFF |
| a | `MIP_TREE_RESTART=force+MIP_TREE_RESTART_GUARD=kC` | tree_restart_guard_20261106 | 58問×種1・2: SGM 15.37→14.92 (−2.9%、86→86)、misc07・timtab1 は種で向きが変わる。10問600秒で解けなくなった問題0 | 得ははっきりしない (速くなった問題は再スタート0〜1回で揺れの見込み)、sct2・csched008 で暫定解が悪くなる回 | TREEEFF |
| a | `MIP_LP_INT_BOUNDS=1+MIP_PROOF_SIDE_REDUCE=dense+MIP_PROOF_CMIR=1+MIP_PROOF_CMIR_POOL (full、前回の試作)` | tree_replay_20261022 | なぞりの上でも 3941〜10569 ノード (効かない)。プールが上限で1本も入っていなかった | 効かない |  |
| a | `MIP_PROBE_CLIQUE` | twovar_20261020 | physiciansched6-2 同値 8→1084組、前処理後 11896×5991→9868×4977、根の LP 4.8→2.1秒。40問で同値が増えるのは2問、悪化は見えない | PROBE_EQUIV と組で既定の候補 (40問で確かめる) |  |
| a | `MIP_DOMCOL` | twovar_20261020 | pcsd 60秒2回: fiber 根 390913〜392977→400577、30n20b8 根 138.4→150.0。air03・nw04 で前処理 +0.4〜0.6秒 | 候補だが優先は低め (最適解を減らす縮約で、40+16問・対称性との確認が要る) |  |
| a | `MIP_SPARSIFY` | twovar_20261020 | 40問の多くで行を書き換えるが効果も悪化もはっきりしない。physiciansched6-2 で単独では FJ を通さない | 候補にしない (HiGHS と同じ選び方にするまで) |  |
| a | `MIP_PROBE_CLIQUE+MIP_SPARSIFY+MIP_DOMCOL (pcsd)` | twovar_20261020 | 60秒2回: 10teams 40.1・18.6→27.5・27.2、neos-860300 31→28、timtab1 暫定解・下界やや悪い、rout 8.6・9.4→10.9・10.4。最適値はすべて一致 | (個別に判断、上の3行) |  |
