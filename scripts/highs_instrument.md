# HiGHS の計測用パッチ (`highs_instrument.patch`)

HiGHS 1.15.1 (`git clone --depth 1 --branch v1.15.1 https://github.com/ERGO-Code/HiGHS.git`) に当てて
`cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DFAST_BUILD=ON .. && ninja highs-bin` でビルドする。標準エラーに出すもの:

- `HCONF c|r 長さ 0-1 一般整数 連続`: 衝突プールに入れた衝突 (c: 衝突、r: 再収束)
- `HCA prop|proof ...`: 衝突解析の呼び出し (説明の長さ、上限)
- `HCHG` / `HINF`: 境界の変更・矛盾の理由ごとの数 (終了時)
- `HSEP round k obj v sep0= sep1= sep2=`: 分離のラウンドごとの LP 値と分離器ごとのカット数 (tableau / path / mod-k)
- `HGEN` / `HMIX`: 経路集約の CMIR・path mixing cut
- 環境変数 `HIGHS_NO_MIX`: path mixing cut を止める
- `HADD 0 obj v lprows r`: 分離のラウンドでカットプールから LP に加えて解き直した後の LP 値と LP の行数 (行数の増分が
  そのラウンドで LP に入ったカットの数の目安。`HADD` の最初の数は `addCuts` の後なので常に 0)
- 環境変数 `HIGHS_SKIP_SEP=数字の並び`: 分離器を止める (0: tableau、1: path、2: mod-k。例 `HIGHS_SKIP_SEP=02`)
- 環境変数 `HIGHS_NO_IMPLB` / `HIGHS_NO_CLQ`: implied bound・clique の分離を止める
- 環境変数 `HIGHS_CUT_DUMP=ファイル`: 最初の分離ラウンドの経路集約の集約行と生成したカットを書き出す
  (`presolve=off` で使う)。`HIGHS_CUT_DUMP=ファイル cargo test --release highs_cut_dump -- --ignored --nocapture`
  でこちらの CMIR と比べる
- 環境変数 `HIGHS_TAB_DUMP=ファイル` (`HIGHS_TAB_DUMP_CALL=k`、既定 1): 本体の MIP の tableau の分離の k 回目の呼び出しの LP の状態・
  集約行・カットを同じ形式で書き出す (行の後ろに整数の行かどうか `I 0|1`、`F` 候補の数とそのうち行のスラックの数、`V` 集約した
  基底変数 (行のスラックか) とスコア、`O` LP 値)。上のテストでそのまま比べられる
- `HPROP 数 obj v`: 分離のラウンドの後の伝播で変わった境界の数と LP 値
- 環境変数 `HIGHS_ADD_DUMP=ファイル`: 本体の MIP の最初の 3 回の、プールから LP に入れたカット (`A 回 右辺 長さ 列 係数 ...`) を書き出す
- 環境変数 `HIGHS_TAB_NO_SLACK`: tableau の分離で、整数の行のスラックを候補にしない
- 上のテストは libpython にリンクしないと通らない (pyo3 の extension-module のため)。例:
  `CARGO_TARGET_DIR=/tmp/tgt RUSTFLAGS="-L $(python3 -c 'import sysconfig;print(sysconfig.get_config_var("LIBDIR"))') -C link-arg=-lpython3.13"`。
  `HIGHS_CUT_DUMP_ONLYH=k` で HiGHS だけがカットを作れた集約行を詳しく表示する

## 木の計測を足したもの (`highs_instrument_tree.patch`、2026-10-20 neos-2978193-inde の調査)

`highs_instrument.patch` を含む v1.15.1 からの差分 (こちらだけ当てればよい)。追加分:

- 環境変数 `HIGHS_TREE_LOG`: 本体の MIP の木 (サブ MIP・ヒューリスティクスの潜りは除く) のノードごとに
  `HNODE d=深さ(根=1) r=結果 lp= lb= rows=LP の行数 inbr=強分岐中か` (結果: 0 LP の目的値で打ち切り、1 定義域の矛盾、
  2 LP が実行不能、4 下界で刈る、5 開いたまま (分枝へ))、分枝ごとに `HBR d= col= val= up= degen=退化の係数`、
  双対証明から衝突カットを作るごとに `HGC ok pre 証明の長さ precont その連続列の数 cut カットの長さ ...` (`fail`・`prefail` は作れない)
- 環境変数 `HIGHS_CONF_W=w`: 分枝スコアの衝突の項の重み (既定 1e-2、0 で衝突スコアを使わない)
- 環境変数 `HIGHS_NO_GENCONF`: 双対証明・Farkas 証明から衝突カット (`HighsCutGeneration::generateConflict`) を作らない
- 集計: `work_tree/tree_stats.py` (葉の終わり方・分枝した列・深さごとの数。ENOMOTO の `ENOMOTO_MIP_TREE_LOG` の
  `ENODE`・`EBR` も読む)、`work_tree/depth_lp.py` (深さごとの分枝したノードの LP 値の平均)
