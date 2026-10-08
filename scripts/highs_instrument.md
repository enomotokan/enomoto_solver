# HiGHS の計測用パッチ (`highs_instrument.patch`)

HiGHS 1.15.1 (`git clone --depth 1 --branch v1.15.1 https://github.com/ERGO-Code/HiGHS.git`) に当てて
`cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DFAST_BUILD=ON .. && ninja highs-bin` でビルドする。標準エラーに出すもの:

- `HCONF c|r 長さ 0-1 一般整数 連続`: 衝突プールに入れた衝突 (c: 衝突、r: 再収束)
- `HCA prop|proof ...`: 衝突解析の呼び出し (説明の長さ、上限)
- `HCHG` / `HINF`: 境界の変更・矛盾の理由ごとの数 (終了時)
- `HSEP round k obj v sep0= sep1= sep2=`: 分離のラウンドごとの LP 値と分離器ごとのカット数 (tableau / path / mod-k)
- `HGEN` / `HMIX`: 経路集約の CMIR・path mixing cut
- 環境変数 `HIGHS_NO_MIX`: path mixing cut を止める
- 環境変数 `HIGHS_CUT_DUMP=ファイル`: 最初の分離ラウンドの経路集約の集約行と生成したカットを書き出す
  (`presolve=off` で使う)。`HIGHS_CUT_DUMP=ファイル cargo test --release highs_cut_dump -- --ignored --nocapture`
  でこちらの CMIR と比べる
