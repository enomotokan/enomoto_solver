# 問題の特徴で入り切りする作り直し (gate2、2026-11-17、利用者の依頼「並列で」)

過去に不採用・保留とした試作のうち、全体では基準に届かなかったが、問題ごとには「全種で速い問題」と「全種で遅い問題」が
はっきり分かれたものを、問題の特徴の連続 (シグモイド) 関数で入り切り / 強さを決める形に作り直す。1 セッション 1 件。

## 共通の約束

- 返答・コメント・コミット・記録は日本語 (リポジトリの既存の書き方に合わせる)。作業ブランチは ccr-1beed81b-ex0uhq のみ
  (push 拒否時は fetch+merge、rebase・force-push はしない)。PR は作らない。報告は親セッション
  session_01HrNYnHHEAD5vXdU4MHQrtc に send_message で、日本語で。
- 利用者の方針: パラメータは問題の特徴の連続 (シグモイド) 関数。予算は時間でなく作業量で。
- 採用基準: SGM の 95% 区間の上限 ≲ +5%、解けた数を減らさない、全種で 1.25 倍超の遅れなし、誤答 0、cargo test 通過。
- 既定は変えない (環境変数つきの試作)。cargo test は
  `LD_LIBRARY_PATH=/usr/local/lib cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]'`。
- 試作が古いコードの上にある場合は、今の既定 (0.1.9 + rtc + LPINF の確かめ) の上で動くか先に確かめ、壊れていれば直す。
- 進め方:
  1. 記録 (下の「場所」) と既存の結果・ログから、効く群と遅れる群を分ける特徴を数える。まず分けられる見込みを親に短く報告
     (分けられないなら正直にそう言って区切る。結果を見て選んだしきい値は楽観的なので、後で別の種で確かめる前提)。
  2. 見込みがあれば試作 → cargo test と fuzz (既存の fuzz 台本、例 analysis/release019_20261117/fuzz.sh) で誤答 0 →
     手元で効く/遅れる代表問題を 1〜2 種で当たる。
  3. analysis/gate2_20261117/<名前>/ に cfg・plan・run・summ・README (analysis/slow2_20261117 の台本の写しでよい) を作り、
     しきい値を選ぶのに使っていない種 (P = MIPLIB 58 問 × 種 4〜6・60 秒、M = slow2 の M 48 問 × 種 3・4・600 秒、
     必要なら効く問題を足す) の計測の依頼 (分担数と 1 分担の見積り) を親に送る。計測は親が空いたセッションに割り振る。
- 他の gate2 のセッションも同じブランチで並行して作業する。自分の試作の環境変数とファイルだけを触り、共有のコード
  (separation.rs・solver.rs など) を直すときは差分を小さく、push 前に fetch+merge してビルドとテストを通すこと。

## 候補 (番号 = セッション)

1. TAB_HIGHS_RULES (`ENOMOTO_MIP_TAB_HIGHS_RULES=1`、tableau の行の選び方を HiGHS に)。analysis/revisit_20261112/summ_P.txt、
   analysis/revisit_20261112.md:32,69、analysis/gap4_rootstuck_mem_20261109.md:324-332。P −3.3% [−10.5, +5.1]、メモリ −12%。
   速い: eil33-2・neos-827175・neos-860300・neos-911970・neos17・qnet1・hypothyroid-k1。遅い: neos-960392・nw04・pk1・sp150x300d・
   roll3000 (22→38.5)・30n20b8 (下界 140→128)。手がかり: 密な tableau カットで下界を得る問題で損 (カットの密度・長いカットの伸びへの寄与)。
2. CLQPRB (`ENOMOTO_MIP_CLQPRB=1dgr`、プロービングの含意を後の伝播に)。analysis/prbimp_20261113.md:15-25,155-162、
   analysis/clqprb_20261112.md:41、analysis/cand1114_20261113/README.md:24。M2 −15.3% [−28.4, +0.9]。mzzv11 が 6/6 解ける、
   mzzv42z・neos8 速い。puriri で解を失う (根のヒューリスティクスの順が元)、g 形では timtab1・widden 遅い。含意が残るのは 6 問だけ。
3. 深さ優先寄りの探索 (cand1116f = `ENOMOTO_T_MIP_PLUNGE_QUOT=1`・`SIBLING_QUOT=1` ほか、上限なし dmx、`ENOMOTO_MIP_NODESEL_BANDIT=1`)。
   analysis/cand1116_20261115/summ_M.txt、analysis/treeclose_20261114.md:602-660,717-830、analysis/cand1117_20261116/kekka.md:20-45。
   M −16.4% [−25.5, −6.3]、解けた 60→68。速い: rocI-4-11 0.28・mcsched 0.31・timtab1 0.37・rococoC10 0.55・supportcase33 0.65 ほか。
   遅い: CMS750_4 1.72・supportcase40 1.3〜2.1・tbfp-network。後継の DFSBB=2 は既定済みなので、その上での差分として作る。
4. NODELP2 (`ENOMOTO_MIP_NODELP2=pbxg`、重い LP のスレッドでノードの LP を摂動つきで)。analysis/nodelp2_20261107.md:15-26。
   P +0.9%。ns1830653 が 2 回とも解ける。traininstance6・tr12-30 で下界が悪い。境 (平均ノード LP 反復 g=100) 付近の問題が害。
5. MODKVB (`ENOMOTO_MIP_MODKVB=1`、VUB/VLB で置き換えた mod-k)。analysis/modkvb_20261107.md:25-46,144-150。P +3.7%、解けた 84→79。
   25 問で根の下界が上がる、h80x6320d 時間切れ→37.6 秒。physiciansched6-2 が時間切れ、noswot・mc11 遅い。害の仕組み: 根の 0 ラウンド目に
   450 本入りカットの LP が反復の上限に当たる (上限に当たったら試作のカットだけ外す案)。
6. SPARSIFY_LATE (`ENOMOTO_MIP_SPARSIFY_LATE=1`)。analysis/presolve2_20261111.md:30,116-124。P +1.0%、解けた 127→124。kasai が解ける。
   air03・nubu 遅い、nw04 RSS 2.7 倍。手がかり: 疎化で行が消える問題だけに絞る。
7. SYM_OF (`ENOMOTO_MIP_SYM_OF`、orbital fixing)。docs/improvement_history.md:10131-10140,10402。neos5 時間切れ→50 秒・drayage・nw04 速い、
   qiu・physiciansched6-2 遅い (下界)。手がかり: lex-leader の行が根の下界を強めるか。今の既定は SYMDYN=f (orbital fixing + LexRed) なので、
   その上で lex の行の有無を特徴で切り替える形として作る。
8. CUTCONF (`ENOMOTO_MIP_CUTCONF=1`) と INOUT_SEP (`ENOMOTO_MIP_INOUT_SEP=1`)。analysis/cutconf_20261110.md:11,171-172,223-224、
   analysis/winwhy_20261111.md:193、docs/improvement_history.md:9788-9801、analysis/inout_sep_20261022.md:66,95-113。
   CUTCONF: qiu 速い、10teams・neos5・mas74・mas76 遅い (制約の長さ・一般整数の割合・刈りと締めの比で分ける案)。
   INOUT_SEP: 根の下界は上がるが 40 問で悪い。最適面の広さ (x_c と x_lp の差)・GATE=2。
9. CUTSEL_DCD (`ENOMOTO_MIP_CUTSEL_DCD=1` + `DCD_T=1`) と SEP_FAST + FACE_ADAPT (`ENOMOTO_MIP_SEP_FAST=1`・`ENOMOTO_MIP_FACE_ADAPT=1`)。
   docs/improvement_history.md:9880-9886,9901-9915、analysis/dircutoff_20261022.md、analysis/revisit_20261112/summ_P.txt。
   DCD: neos-911970 26→13・10teams 18→4、mik-250・mc11・neos-960392・noswot・timtab1 遅い (LP の頂点が退化した面に留まる問題で効く)。
   FACE: 10teams 27→5・drayage・h80x6320d・rout 速い、noswot・qiu・mc11 遅い (面の LP が重い問題で効く)。
10. HO3_ENUM (`ENOMOTO_MIP_HO3_ENUM=1`)・SMALL3_SUBMIP_LIGHT (`ENOMOTO_MIP_SMALL3_SUBMIP_LIGHT=1`)・LOCALMIP+CHAIN (`ENOMOTO_MIP_LOCALMIP=1`・`_CHAIN=1`)。
    analysis/presolve2_20261111.md:31,118-123、analysis/small_overhead3_20261102.md:22-27,123-169、docs/improvement_history.md:9978-9991。
    HO3: h80x6320d・physiciansched6-2 が解ける、neos8・sp150x300d 遅い (列挙の時間)。SMALL3: 10teams・nubu 速い、physiciansched6-2・qiu 時間切れ。
    LOCALMIP: 30n20b8・neos-1456979 の解が早い、neos-960392 落とす (FJ が違反の少ない点で終わる問題だけで使う案)。
11. SEPA_MCF (`ENOMOTO_MIP_SEPA_MCF=auto`)・LIGHT_RESTART_ROOT / TREE_RESTART・PRESOLVE_PARALLEL_INT。docs/improvement_history.md:10034-10038,10115-10129,10183、
    analysis/mcf_cuts_20261028.md、analysis/rootgap_20261116/clq/README.md:160-172。MCF: ta2-UUE が解ける・tavua・fiber・beavma、h80x6320d 遅い。
    TREE_RESTART: mc11 が解ける (csched008 の HiGHS の in-tree restart とも関係)、neos-860300・nubu で損。PARALLEL_INT: irp・mzzv11・noswot 速い、
    traininstance6 1.77 遅い (SYM_TWINS でほぼ取れているので優先は低い)。

## 追記 (2026-11-18、利用者の方針の訂正)
- 「なんでもシグモイドにするのではなく、普通の if で充分ならそれでいい」。特徴で入り切りするとき、効く問題と効かない問題がはっきり分かれ境目の近くに問題がないなら、しきい値の if で書く。境目の近くに問題が多い・効き目が特徴とともに徐々に変わる場合にだけ連続 (シグモイド) にする。
