# Netlib forplan の「crash」の調べ (2026-11-23、利用者の依頼)

## 0. 結論

- **crash ではなく、当方の MPS の読み込み (`src/mps.rs` の `parse_mps`) のエラー。** 求解は正しい。
  - `python -m enomoto_solver.benchmark_highs --only forplan` の結果の `ours_error` は `MPS: line 6: the row "DEDO3" is defined twice`。
    Python の例外として記録され、表では `nan` と `[?]` になる (panic・SIGSEGV ではない。RUST_BACKTRACE も出ない)。
  - forplan は**名前に空白を含む固定欄形式** (ROWS の名前が `DEDO3 1R`・`DEDO3 2R` … で、5〜12 桁目の 8 文字が名前)。当方の読み込みは空白区切りの自由形式で読むので、
    名前が `DEDO3` に切れ、2 本目で重複として止まる。`src/mps.rs` の先頭の注釈に「名前に空白を含む固定欄形式は読めない」と書いてある既知の制限。
  - HiGHS で読んで当方に渡すと (計測の台本の以前の読み方 `_load_lp` + `_build_our_model`)、当方の求解は 0.024 秒で最適 −664.218961272205 (HiGHS −664.2189612722054)。
- **いつから**: 2026-11-20 の a7e9d28e8「計測の台本のモデル読み込みをこのクレートの MPS 読み込みに切り替え」から。それより前の benchmark_highs は
  HiGHS で読んだモデルを当方に渡していたので、9 月の記録 (overall_final_20260924.md など) では解けていた。当方の MPS の読み込みは同じ日の d0e1cb628
  (`Model.readModel` を追加) で入った。求解の側の後退ではないので、git bisect は要らない。
- ほかの Netlib 92 問は当方の読み込みで読め、目的値は HiGHS と一致する (analysis/proofcost_20261123/adopt/netlib.csv)。名前に空白があっても重ならずに
  黙って読み違える問題は、92 問の目的値が一致したので今のところない。

## 1. 直しの案 (既定の求解は変えない。直すのは親の確認の後)

1. **`src/mps.rs` に固定欄形式の読み直しを足す (推奨)**: 自由形式で読んで「名前の重複」「欄の数が合わない」などで失敗したら、固定欄の桁
   (種類 2〜3、名前 5〜12、15〜22、数値 25〜36、名前 40〜47、数値 50〜61) で読み直す。HiGHS も自由形式 (`HMpsFF`) で読めなければ固定形式 (`HMPSIO`) に戻る。
   自由形式で読める今のファイルは何も変わらない。forplan の頭の数行の単体テストを足す。
2. **計測の台本だけ直す**: `benchmark_highs._read_our_model` で `read_mps` が失敗したら、従来の HiGHS 経由 (`_load_lp` + `_build_our_model`) で組み立てる。
   `Model.readModel` を使う利用者には効かないので、1. の方がよい。

## 2. 直した (2026-11-23、利用者の判断で案 1)

- `src/mps.rs`: `parse_mps` は自由形式で読み (`parse_mps_with(text, false)`、今までと同じ)、エラーのときだけ、区切りでないデータの行を固定欄の桁
  (`fixed_fields`: 2〜3・5〜12・15〜22・25〜36・40〜47・50〜 桁、前後の空白を除き、空の欄は落とす) で読み直す。読み直しでも読めなければ自由形式のエラーを返す。
  自由形式で読めるファイルは読み直さないので結果は変わらない。
- 単体テスト `mps::tests::fixed_format_with_spaces_in_names` (forplan の形の小さな本文: 名前に空白のある ROWS・COLUMNS・RHS・RANGES・BOUNDS と、
  自由形式で読める本文は読み直さないこと)。cargo test --release --lib 420 通過・0 失敗。
- 読みの結果の比べ (`check/readcmp.py`、`check/read_old.jsonl`・`read_new.jsonl`): `.miplib_cache` の 203 本と Netlib 93 本の計 296 本で、
  直す前の wheel (e22e59b8c のビルド) と直した後で、変数の境界・整数性・目的・名前・行数・非零の指紋が forplan 以外すべて同じ。forplan だけ
  読めない → 読める (161 行 + 目的、非零 4564)。`scripts/check_mps_reader.py` でも forplan は HiGHS 経由の組み立てと同じ (`same 1 / 1`)。
- Netlib 93 問 (`check/netlib.csv`・`netlib.out`、`python -m enomoto_solver.benchmark_highs`): 93 / 93 で最適、目的値は全部 HiGHS と一致。
  forplan は 0.012 秒で −664.2189612722 (HiGHS −664.2189612722054)。ほかの 92 問は状態が同じで、13 問は目的値が最後の 1〜2 桁 (相対 1e-15 程度) だけ
  前回 (analysis/proofcost_20261123/adopt/netlib.csv) と違う。読みの指紋は同じなので、読み込みの変更ではなく求解側の揺れ。
