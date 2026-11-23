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
