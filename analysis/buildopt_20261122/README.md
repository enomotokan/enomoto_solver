# buildopt_20261122: 作り方 (LTO・PGO・target-cpu・floor) の見直し

ソースは変えず、作り方だけを変えた版を並べて 1 スレッドで比べた。Cargo.toml の既定は変えていない (ここに書くのは結果と案だけ)。

- 今の作り方: Cargo.toml に `[profile.release]` が無い → LTO なし、codegen-units 16、target-cpu なし (x86-64 の基本の命令だけ)。
- 版は環境変数だけで変える (`CARGO_PROFILE_RELEASE_LTO=fat`、`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`、`RUSTFLAGS=-C target-cpu=...`、`-Cprofile-generate/-Cprofile-use`)。
  panic="abort" は使わない。
- どの版も同じソース (e2b2f0677 を固めた作業木 /home/user/bo_wt) から作った。ff・ff_lto だけ、その上に `apply_ffloor.py` を当てた作業木 (/home/user/bo_wt_ff)。

## 0. 機械と道具

- Intel Xeon @ 2.10GHz、4 コア (1 コア 1 スレッド)、L2 8 MiB、L3 260 MiB。
- CPU の旗: fma sse4_1 sse4_2 movbe popcnt avx f16c abm bmi1 avx2 bmi2 avx512f avx512dq avx512cd avx512bw avx512vl
  (x86-64-v3 まで揃い、v4 の旗もある)。
- rustc 1.97.0、maturin 1.16.0。PGO の llvm-profdata は rustup の llvm-tools (LLVM 22) のもの (系の llvm-profdata 18 は古くて読めない)。

## 1. 版と、作る時間・大きさ

| 版 | 中身 | 作る時間 [秒] | wheel [バイト] | .so [バイト] |
|---|---|---:|---:|---:|
| base | 今の既定 | 128 | 7,429,106 | 27,619,832 |
| v2 | target-cpu=x86-64-v2 | 125 | 7,418,975 | 27,656,672 |
| v3 | target-cpu=x86-64-v3 | 131 | 7,498,268 | 27,681,952 |
| lto | LTO fat + codegen-units 1 | 384 | 6,822,373 | 22,791,600 |
| lto_v3 | lto + v3 | 378 | 6,940,483 | 22,981,064 |
| ff | base + 速い floor (下の 3 節 (a)) | 198 | 7,469,061 | 27,680,992 |
| ff_lto | ff + lto | 439 | 6,893,105 | 22,930,528 |
| pgogen | lto + -Cprofile-generate (学習用) | 414 | 10,799,491 | 36,586,776 |
| pgo | lto + PGO | 420 | 6,859,456 | 20,885,976 |
| pgo_v3 | lto + PGO + v3 | 444 | 7,004,891 | 21,309,096 |
| dual | base の .so と v3 の .so を両方入れ、`__init__.py` で CPU を見て選ぶ (3 節 (c)) | (base + v3) | (下の 3 節) | 2 本 |

- 時間は `maturin build --release` の壁時計 (4 コア、target を空にしてから)。ff が base より長いのは src の 55 ファイルを触って作り直しが広いため。
- LTO で .so は 17% 小さく、PGO でさらに 8% 小さい。作る時間は LTO で約 3 倍、PGO は学習を入れて lto の 2 倍強
  (pgogen の作り 414 秒 + 学習 約 90 秒 + pgo の作り 420 秒)。
- PGO の学習 (`pgo_train.sh`): 計測に使わない 10 問 (neos17, misc07, binkar10_1, eil33-2, graph20-20-1rand, neos-827175,
  hypothyroid-k1, p200x1188c, nu25-pr12, dcmulti) を pgogen で 1 スレッド、30 秒ずつ。

作り方: `SRC=作業木 sh build_var.sh 版` (→ var/版/enomoto_solver/ と var/版/build.txt)。通しは `build_all2.sh`。

## 2. 1 スレッドの比べ (m1)

`python run_var.py m1 base v2 v3 lto lto_v3 ff ff_lto pgo pgo_v3 dual`: 15 問 × 10 版 × 2 回、1 スレッド、決定的、種 1、上限 200 秒。
問題ごとに版の順を回した。結果は res_m1.jsonl、解析は `python ana_m1.py` → ana_m1.txt。

**道筋: 300 回すべてで、状態・ノード数・目的値が base と同じ** (違い 0 件)。作り方を変えても同じ探索をしている。

時間の比 (版/base、2 回のうち速い方どうし):

| 問題 | base [秒] | v2 | v3 | lto | lto_v3 | ff | ff_lto | pgo | pgo_v3 | dual |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| air03 | 4.9 | 1.002 | 1.019 | 0.989 | 0.958 | 1.056 | 0.969 | 0.915 | 0.896 | 1.005 |
| neos8 | 6.3 | 0.964 | 0.949 | 0.970 | 0.946 | 0.964 | 0.955 | 0.893 | 0.872 | 0.975 |
| nw04 | 18.9 | 0.949 | 0.944 | 1.046 | 0.961 | 1.020 | 1.019 | 0.891 | 0.878 | 0.966 |
| physiciansched6-2 | 30.1 | 0.952 | 0.962 | 1.022 | 0.999 | 1.052 | 1.029 | 0.978 | 0.953 | 0.981 |
| neos-860300 | 25.8 | 0.981 | 0.981 | 0.991 | 0.920 | 1.002 | 0.988 | 0.935 | 0.911 | 0.988 |
| h80x6320d | 45.2 | 0.893 | 0.902 | 1.002 | 0.825 | 1.006 | 0.997 | 0.961 | 0.811 | 0.893 |
| exp-1-500-5-5 | 6.9 | 0.918 | 0.952 | 0.934 | 0.911 | 0.980 | 0.916 | 0.897 | 0.826 | 0.905 |
| qnet1 | 5.2 | 0.861 | 0.853 | 0.973 | 0.821 | 0.998 | 0.993 | 0.961 | 0.793 | 0.883 |
| markshare_4_0 | 8.8 | 0.947 | 0.959 | 0.951 | 0.915 | 1.032 | 0.988 | 0.857 | 0.862 | 0.977 |
| mas76 | 37.8 | 0.956 | 0.970 | 1.002 | 0.917 | 0.989 | 0.958 | 0.936 | 0.891 | 0.983 |
| mik-250-20-75-4 | 14.3 | 0.932 | 0.950 | 0.986 | 0.915 | 0.979 | 1.009 | 0.942 | 0.872 | 0.945 |
| rout | 12.0 | 0.900 | 0.926 | 0.965 | 0.873 | 0.990 | 0.970 | 0.946 | 0.846 | 0.913 |
| noswot | 24.5 | 0.973 | 0.965 | 0.963 | 0.947 | 0.982 | 0.993 | 0.915 | 0.923 | 0.981 |
| neos-911970 | 26.0 | 0.964 | 1.000 | 1.012 | 0.983 | 1.020 | 0.989 | 0.951 | 0.926 | 0.981 |
| qiu | 34.1 | 0.966 | 0.976 | 0.967 | 0.928 | 0.969 | 0.950 | 0.916 | 0.911 | 0.959 |
| **幾何平均** | | **0.943** | **0.953** | **0.984** | **0.920** | **1.002** | **0.981** | **0.926** | **0.877** | **0.955** |

同じ版の 2 回の揺れは |log(rep1/rep0)| の平均で 2.4〜4.4%。そのため 1 問ごとの ±5% は揺れの内にある。

読み:
- **pgo_v3 が一番速く、幾何平均 0.877 (12% 減)**。15 問すべてで 1 未満 (最大 0.953)。
- target-cpu だけで 5〜6% 減る (v2 0.943、v3 0.953)。この 15 問では v2 と v3 の差は揺れの内。
- LTO だけでは 1.6% 減 (0.984) と小さい。ただ v3 と重ねると 0.920 になり、足し算 (0.984×0.953=0.938) より効く。
- PGO は lto に重ねて 6% 減 (0.926)。v3 と重ねると 0.877。
- dual (CPU を見て v3 の .so を読む) は 0.955 で、v3 (0.953) と同じ。読み分けに損は無い。

## 3. floor の 3 案

親セッションのホットスポット調べで、base では `f64::floor` が libm の関数呼び出しになっていた (x86-64 の基本の命令に丸めの命令が無いため。SSE4.1 の roundsd は v2 から使える)。

### (a) 持ち運べる速い floor (ff)

`apply_ffloor.py` が src/ffloor.rs (trait `FFloor`) を足し、`.floor()/.ceil()/.round()` の 474 か所 (55 ファイル) を `.ffloor()/.fceil()/.fround()` に置き換える。

- |x| < 2^52 のとき `t = x as i64 as f64` を作る。floor は `t > x` なら `t - 1`、ceil は `t < x` なら `t + 1`。round は差 `d = x - t` で ±0.5 を見る。
- 結果が 0 のときは x の符号を写す (-0.0 を守る)。
- それ以外 (|x| ≥ 2^52、NaN、±inf) は std に任せる。
- 単体テスト `same_as_std`: 40 万値 (±0.0、NaN、±inf、2^52 前後、半端な ±0.5、乱数) で std とビット一致。`cargo test` で通った。

結果: **ff は幾何平均 1.002、ff_lto は 0.981 (lto 0.984 と同じ)**。速くならなかった。ノード数・目的値は同じ。

floor の呼び出しは、glibc の floor が SSE4.1 の ifunc で roundsd 1 つになっており、もともと安い。整数への往復 (cvttsd2si + cvtsi2sd + 比べ) は、その呼び出しと同じくらいかかる。v2・v3 の 5〜6% は floor 以外 (広い所のベクトル化・popcnt・bmi など) から来ていると見る。
ここは perf で確かめてはいない。ただ ff が効かず v2 が効くことと、v2 の効きが 15 問のほぼ全部に広く出ている (0.86〜1.00) ことから、そう読める。

### (b) `#[target_feature]` と `is_x86_feature_detected!` で、関数ごとに実行時に切り替える

**作っていない。** (a) から floor が主な理由ではないと分かったので、熱い関数を少し選んで v3 版を持つ形では、v2・v3 の 5〜6% の大半を取れない見込み。
効きが広く散っているので、取るには .so 全体を切り替える (c) の方が合う。

関数ごとに切り替える場合の注意は 2 つ:
- 関数ごとに分岐がいる。
- `#[target_feature(enable = "avx2,fma,...")]` の関数から呼ぶ関数は、`#[inline]` で取り込まれない限り base の命令のまま。
  熱い所が深い呼び出しの奥にあると効かない (`multiversion` crate でも同じ)。

### (c) 1 つの wheel に base と v3 の .so を両方入れ、`__init__.py` で選ぶ (dual)

`make_dual.py BASE V3 OUT` が var/OUT を作る。v3 の .so を `_core_v3*.so` として足し、`__init__.py` の頭に `_enomoto_load_core()` を足す。

`_enomoto_load_core()` の動き:
- /proc/cpuinfo の flags に avx avx2 bmi1 bmi2 fma f16c movbe abm (x86-64-v3) が揃っていれば、`_core_v3` を `enomoto_solver._core` という名前で読む
  (importlib の `spec_from_file_location` を使い、sys.modules に入れる)。v3 の .so の中の `PyInit__core` はそのまま使える。
- 揃わなければ、普通の `_core` を読む。
- `ENOMOTO_CORE=base|v3|auto` で強制できる (既定は auto)。
- 他のモジュールが `_core` を import するより前に動かす必要がある (`from .constraint import` の前に置く)。

wheel の試作は `make_dual_wheel.py BASE_WHEEL V3_WHEEL INIT_PY OUT_DIR`。maturin で 2 回作った wheel から、1 本目に 2 本目の .so と新しい `__init__.py` を入れ、RECORD (sha256・大きさ) を作り直す。

- pgo + pgo_v3 で作った wheel は 13,521,436 バイト (pgo だけの wheel 6,859,456 の約 2 倍)。展開後は .so 2 本で約 42 MB。
- `pip install` で入った。ENOMOTO_CORE が auto → `_core_v3`、base → `_core`、v3 → `_core_v3` を読み、`__version__` も読めた。
- 1 スレッドの時間は v3 と同じ (2 節の dual、0.955)。

配り方:
- CI で maturin を 2 回回す (`RUSTFLAGS` を変えて、別の target dir に)。2 回目の .so を 1 回目の wheel に入れ直す小さな手順がいる (上の make_dual_wheel.py の形)。
- maturin だけでこの形を作る機能は無い。
- 作る時間は 2 倍。PGO を使うなら、学習は 1 回で両方に使える (pgo_v3 は pgo と同じ merged.profdata を使い、作るときに profile の警告は出なかった)。
- PyPI の wheel の名札 (タグ) には CPU の命令の水準を書く欄が無い。v3 だけの wheel を出すと v3 の無い CPU で SIGILL になるので、出せるのは base だけか dual。
- ここの wheel は手元の glibc で作ったので manylinux_2_39 になっている。配るときは今まで通り manylinux のコンテナで作る (auditwheel も .so 2 本を見る)。
- macOS (arm64) と Windows は別の話。arm64 には v3 は無く、そのまま base 1 本でよい。Windows では /proc/cpuinfo が無いので、今の試作は base を読む。
  Windows でも切り替えるなら、Rust 側に `is_x86_feature_detected!` で v3 に対応しているかを返す小さな関数を足すか、cpuid を読む。

## 4. 誤答の確かめ (lto・pgo_v3)

<!-- FUZZ -->

## 5. 案

<!-- PROPOSAL -->

## 6. ファイル

- `build_var.sh`、`build_all.sh` (1 回目、作業木を固める前で使わない)、`build_all2.sh` (今回の全部)、`build_all.out`・`build_all2.out`
- `pgo_train.sh`、`res_pgotrain.jsonl` (学習の回)
- `apply_ffloor.py` ((a) の当て方)、`make_dual.py`・`make_dual_wheel.py` ((c))
- `run_var.py` (1 スレッドの比べ)、`res_m1.jsonl`・`run_m1.out`、`ana_m1.py`・`ana_m1.txt`
- `fuzz_var.sh` (版ごとの fuzz 7 本)、`fuzz_lto.out`・`fuzz_pgo_v3.out`、`cargo_lto.log`・`cargo_pgo_v3.log`
- `plan_P.py`・`run_P.py` (4 スレッド P の比べ、まだ回していない。4 節の後)
- var/・target_var/・pgo/ はコミットしない (.gitignore)
