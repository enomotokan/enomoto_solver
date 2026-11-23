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

wheel の試作は `make_dual_wheel.py BASE_WHEEL V3_WHEEL INIT_PY OUT_DIR` (このときの形。今は 7 節の scripts/make_dual_wheel.py に移した)。maturin で 2 回作った wheel から、1 本目に 2 本目の .so と新しい `__init__.py` を入れ、RECORD (sha256・大きさ) を作り直す。

- pgo + pgo_v3 で作った wheel は 13,521,436 バイト (pgo だけの wheel 6,859,456 の約 2 倍)。展開後は .so 2 本で約 42 MB。
- `pip install` で入った。ENOMOTO_CORE が auto → `_core_v3`、base → `_core`、v3 → `_core_v3` を読み、`__version__` も読めた。
- 1 スレッドの時間は v3 と同じ (2 節の dual、0.955)。

配り方:
- CI で maturin を 2 回回す (`RUSTFLAGS` を変えて、別の target dir に)。2 回目の .so を 1 回目の wheel に入れ直す小さな手順がいる (上の make_dual_wheel.py の形)。
- maturin だけでこの形を作る機能は無い。
- 作る時間は 2 倍。PGO の学習も、base と v3 で別々に要る (7.3 節。ここで「1 回で両方に使える、警告は出なかった」と書いたのは誤り。build_var.sh が警告の行を捨てていて見えていなかった)。
- PyPI の wheel の名札 (タグ) には CPU の命令の水準を書く欄が無い。v3 だけの wheel を出すと v3 の無い CPU で SIGILL になるので、出せるのは base だけか dual。
- ここの wheel は手元の glibc で作ったので manylinux_2_39 になっている。配るときは今まで通り manylinux のコンテナで作る (auditwheel も .so 2 本を見る)。
- macOS (arm64) と Windows は別の話。arm64 には v3 は無く、そのまま base 1 本でよい。Windows では /proc/cpuinfo が無いので、今の試作は base を読む。
  Windows でも切り替えるなら、Rust 側に `is_x86_feature_detected!` で v3 に対応しているかを返す小さな関数を足すか、cpuid を読む。

## 4. 誤答の確かめ (lto・pgo_v3)

`bash fuzz_var.sh 版` で確かめた。中身は gainstall_20261122/fuzz_w.sh と同じ 7 本で、環境変数は既定のまま。PYTHONPATH=var/版 にして、`_core` がその版から来ることを始めに確かめる。
cargo test は作業木 /home/user/bo_wt で回した。その版と同じ環境変数 (LTO fat・CGU 1、pgo_v3 はさらに v3 と profile-use) を付けて `cargo test --release --lib`。

| 確かめ | lto | pgo_v3 |
|---|---|---|
| fuzz_tiny 400 0 | bad 0 | bad 0 |
| mip_fuzz_vs_highs 300 | bad 0 | bad 0 |
| fuzz_objint 1000 0 | bad 0 | bad 0 |
| fuzz_slack 1000 0 | mismatches 0 | mismatches 0 |
| fuzz_tiny 400 1000 | bad 0 | bad 0 |
| fuzz_objint 1000 5000 | bad 0 | bad 0 |
| fuzz_slack 1000 5000 | mismatches 0 | mismatches 0 |
| cargo test --release --lib | 419 通過・0 失敗 (19 は ignored) | 419 通過・0 失敗 (19 は ignored) |

誤答・失敗は無い。fuzz_objint の「nodes>0」の数は版によって 1〜5 違う。fuzz は既定のスレッド数 (複数) で回るためで、答えはどれも合っている。
pgo の fuzz は回していない。pgo は pgo_v3 と同じ profile・同じ LTO で、v3 を付けないだけだから。m1 でノード数・目的値が同じことは見た。

## 5. 案

1. **配る wheel は LTO (fat、codegen-units 1) + PGO にする**。
   - 1 スレッドで 7% 減 (pgo 0.926)。道筋は同じで、.so は 24% 小さい。
   - 代わりに、作る時間が 128 秒から約 15 分 (pgogen の作り + 学習 + pgo の作り) になる。
   - LTO だけなら、Cargo.toml に `[profile.release] lto = "fat"`、`codegen-units = 1` を足すだけで済む。ただし LTO だけの効きは 1.6% と小さい。
     手元の `maturin develop` も遅くなるので、Cargo.toml の既定は変えず、配る CI でだけ `CARGO_PROFILE_RELEASE_*` を付ける形を勧める。
   - PGO は、学習の問題と手順 (`pgo_train.sh`) を CI に持ち込む必要がある。ソースが変わると profile に合わない所が出るので、作るたびに学習し直す。
2. **x86-64 では、base と v3 の 2 本入り wheel (dual) にする**。
   - v3 を重ねると、pgo_v3 で 12% 減 (0.877)。読み分けの損は無い (dual 0.955 ≈ v3 0.953)。
   - wheel は約 2 倍になる (pgo + pgo_v3 で 13.5 MB)。作る時間も .so 2 本分かかる。PGO の学習は 1 回で両方に使える。
   - v3 だけの wheel は、v3 の無い CPU で落ちるので出せない。
   - 手間を減らすなら、v3 をやめて v2 (target-cpu=x86-64-v2、2009 年ごろ以降の CPU) を既定にする手もある。この 15 問では v2 が 0.943 で、v3 と同じ。
     ただし、v2 の無い古い CPU では動かなくなる。
3. **floor の (a) は入れない** (速くならない)。(b) の関数ごとの切り替えも、効きが広く散っているので見送る。
4. 4 スレッドの P は 6 節。1 スレッドの差は 4 スレッドでは縮み、ずらし比で見ると 1.6% (pgo_v3) と 1.3% (lto_v3) だけになる。区間はどちらも 1 をまたぐ。

## 6. 4 スレッドの P (base・pgo_v3・lto_v3)

- 内容: MIPLIB の 58 問 × 種 1〜3 × 3 版で 522 回。4 スレッド、上限 60 秒。ソルバーは同時に 1 つで、版の順は問題・種で回した。
  ほかの計測とは並べていない。
- 回し方: `python plan_P.py 1 base pgo_v3 lto_v3; python run_P.py 0` → res_P_s0.jsonl。
  2 時間の上限で 1 回止まったので、同じコマンドで続きから再開した。
- 照合: `python ci.py 60 base 版 res_P_s0.jsonl` (gainstall_20261122/ci.py の写し) と `python ana_P.py base 版 res_P_s0.jsonl`。

| 版/base | ずらし幾何平均の比 (95% 区間) | 解けた数 (base / 版) | 食い違い | 全種で 1.25 倍超の遅れ | 両方解けた回の素の時間比 | ノード数が同じ回 | (ノード+1) の比 (参考) |
|---|---|---|---|---|---:|---|---:|
| pgo_v3 | 0.984 [0.946, 1.023] | 139 / 138 | 0 | なし | 0.924 (138 回) | 74 / 138 | 1.209 |
| lto_v3 | 0.987 [0.968, 1.008] | 139 / 139 | 0 | なし | 0.932 (138 回) | 71 / 138 | 1.200 |
| pgo_v3/lto_v3 | 0.996 [0.971, 1.022] | 139 / 138 | — | なし | | | |

- 解けた数の違い: どちらも mzzv42z だけ。上限の 60 秒の際で解けたり解けなかったりしている。
  - base は種 3 を 58.8 秒で解き、pgo_v3 と lto_v3 は種 3 が上限に達した。
  - lto_v3 は種 2 を 58.3 秒で解き、base は種 2 が上限に達した。
- 両方が解いた回だけを素の時間で比べると、7〜8% 速い。ずらし比 (解けない回を上限の 60 秒として扱い、10 秒ずらす) では短い問題の差が薄まり、1.3〜1.6% になる。
- ノード数の違い: 4 スレッドは非決定なので、ノード数の違いは参考。3 倍を超えて違ったのは次の回。
  - pgo_v3: h80x6320d 種 2、rout 種 1・2、10teams 種 2・3
  - lto_v3: physiciansched6-2 種 2、rout 種 1、10teams 種 2・3
- 10teams の確かめ: base は 3 種とも根で解いた (3.6〜5.8 秒)。pgo_v3 は種 2・3 でノード 1.2〜1.4 万、57〜58 秒かかった。
  そこで、種 2・3 を各版で 4 スレッドで 2 回ずつ、1 スレッドで 1 回ずつ解き直した。
  - 4 スレッド: base 自身の 2 回目 (種 2) が根で解けず、ノード 1.2 万で上限に達した。lto_v3 と pgo_v3 にも根で解けた回がある。
    つまり、根で解けるかどうかは 4 スレッドの回ごとの揺れで、作り方の違いではない。
  - 1 スレッド (決定的): どの版も上限に達し、目的値は同じ (種 2 は 1052、種 3 は 1016)。ノード数は速い版ほど多いだけ (base 6018 / lto_v3 6329 / pgo_v3 6751)。

読み:
- 4 スレッドで時間の比が縮むのは、P の 60 秒の中で揺れの大きい問題 (根で解けるかどうか、上限の際) が比を左右するため。
  1 回あたりの計算は v3・PGO で 7〜8% 速いが、探索の揺れがそれより大きい。
- 答えの食い違いは無く、解けた数も同じ水準 (差は mzzv42z の上限の際の 1 回だけ)。

## 7. 案 C (LTO + PGO + dual) を配るための作り方 (2026-11-23、利用者の判断で案 C に決めた)

手元のビルド (`maturin develop --release`) と Cargo.toml の既定は今のまま。main・develop・PyPI には触らない。

### 7.1 CPU の見分けを Rust 側へ

- src/lib.rs: `_core` に `cpu_x86_64_v3()` と `build_cpu_level()` を出す。
  - `cpu_x86_64_v3()` は、`is_x86_feature_detected!` で x86-64-v3 の全部 (v2 の sse3・ssse3・sse4.1・sse4.2・popcnt・cmpxchg16b と、avx・avx2・bmi1・bmi2・fma・f16c・lzcnt・movbe) を見る。
    avx は OS がレジスタを保存するか (OSXSAVE・XCR0) も含めて見る。x86_64 以外では false。
  - `build_cpu_level()` は、その .so を作ったときの水準 ("x86-64-v3" か "base") を返す。読み分けの確かめ用。
- python/enomoto_solver/__init__.py: `_enomoto_load_core()` を、ほかのモジュールが `_core` を import するより前に動かす。
  - `_core_v3.*` (.so・.pyd) が同梱されていれば、まず基本の `_core` を読み、その `cpu_x86_64_v3()` が真なら `_core_v3` を `enomoto_solver._core` として読み替える。
  - 判定は Rust 側でするので、/proc/cpuinfo の無い Windows・macOS でも同じに動く。
  - `ENOMOTO_CORE=auto|base|v3` で強制できる。v3 を指定して同梱が無ければ、警告して base を使う。ほかの値は警告して auto とみなす。
  - `_core_v3` の無い wheel (arm64、手元の maturin develop) では何もしない。
- 基本の .so も読み込むので、auto で v3 を使うときは .so が 2 つ読まれる。基本の方は判定に使うだけで、そのあとは使わない。
- 確かめ (手元の 2 本入り wheel を pip で入れたもの):
  - auto・空・不正な値 → `_core_v3` (x86-64-v3)。base → `_core`。v3 → `_core_v3`。
  - model.py が使う `_core` も読み替えた方になっている。
  - 同梱の無い wheel で ENOMOTO_CORE=v3 → 警告して base。

### 7.2 PGO の学習に使う問題

前回の学習に使った 10 問 (1 節) は、調べると全部が P の 58 問に入っていた (m1 の 15 問とは重なっていない)。
6 節の 4 スレッドの P は、学習した問題を含んでいたことになる。差は小さい (0.984) ので結論は変わらないが、ここに記しておく。
手元の .miplib_cache の 147 問も、全部が P・M・m1 のどれかに入っている。

そこで、scripts/pgo_train.py で、決まった種から作る問題を学習に使う (repo に入るのは台本だけで、問題のファイルは要らない)。
- 種類: 多次元ナップサック・集合被覆・容量つき施設配置・一般化割当・段取りつきロットサイズ・一般整数・輸送 (LP)・実行不能 (偶奇)。
- 種類ごとに 2 問、1 スレッドと 4 スレッドで、上限 10 秒。手元では 114 秒。
- 木の探索・切除平面・ヒューリスティック・前処理 (HNF を含む)・LP・並列の探索を通す。

### 7.3 台本

- scripts/build_dist_wheel.sh は bash (Linux・macOS・Windows の Git Bash) で、次を順に行う。
  1. LTO fat・codegen-units 1 に `-Cprofile-generate` を付けて作る。
  2. venv に入れて pgo_train.py を解く。
  3. rustup の llvm-profdata で profile をまとめる。
  4. `-Cprofile-use` で base を作る。
  5. x86_64 なら、さらに `-Ctarget-cpu=x86-64-v3` を付けて v3 を作る。
  6. scripts/make_dual_wheel.py で 1 本にする。
- 環境変数: `PGO=0` で LTO だけ。`DUAL=0` で 1 本 (x86_64 以外の既定)。`MATURIN_ARGS`・`PY`・`OUT`・`TRAIN_ARGS` も渡せる。
- scripts/make_dual_wheel.py は、base の wheel に v3 の wheel の `_core.*` を `_core_v3.*` として足し、RECORD を作り直す。
  - 2 本の wheel の名前 (版・タグ) が同じことを確かめる。
  - abi3 の `_core.abi3.so` と Windows の `_core.pyd` にも対応した。

手元 (Linux x86_64、4 コア) で作った時間。1 回目 (build_c.out) は学習 1 回、作り直し (build_c2.out、下の訂正のあと) は学習 2 回:

| 段 | 1 回目 [秒] | 作り直し [秒] | 作り直しの hash mismatch |
|---|---:|---:|---:|
| 計測用の base (LTO + profile-generate) | 573 | 525 | 0 |
| 学習 base (pgo_train.py) | 114 | 110 | — |
| base (LTO + PGO) | 468 | 445 | 0 |
| 計測用の v3 | — | 502 | 0 |
| 学習 v3 | — | 109 | — |
| v3 (LTO + PGO + x86-64-v3) | 479 | 450 | 0 (1 回目は 1282) |
| 合わせて | 1641 (27 分) | 2152 (36 分) | |

今の作り方は 128 秒。2 本入りの wheel は 13,735,281 バイト (1 回目)・13,672,554 バイト (作り直し)。

**訂正 (2026-11-23、親の指摘。analysis/losses_20261123/README.md 3.1 節)**: 上の 1 回目の作り (build_c.out) では、v3 の作りで
`function control flow change detected (hash mismatch)` が **1282 個** 出ていた (base の作りでは 0)。
前に「3 関数で害は無い」と書いたのは、ログの先頭だけを見た読み違い。
- 原因: base の命令の計測用の版で学習した profile を、target-cpu=x86-64-v3 の作りに使ったこと。命令が違うと関数の形 (CFG) が変わる。
- 結果: 当方の関数 (solve_slope_intercept_dual_impl を含む) の profile がほとんど捨てられ、v3 は実際にはほぼ LTO + v3 だった。
- 1〜6 節の pgo_v3 も同じ作り (pgogen は base) なので、同じく v3 の PGO はほぼ効いていない。
  m1 の pgo_v3 0.877 と、6 節の 4 スレッドの P の pgo_v3 は、この状態で測ったもの。
- 直し方: 台本を、base は base の計測用の版で、v3 は v3 の計測用の版で、別々に学習する形にした (学習 2 回)。
  さらに作りのたびに hash mismatch の数を数え、`MAX_MISMATCH` (既定 0) を超えたら止める。直した後の作りは 7.6 節。

### 7.4 release の CI の設計 (変更案は release_proposed.yml。7.7 節で .github に入れた)

| 対象 | 機械 | PGO | dual | 見込みの時間 |
|---|---|---|---|---|
| Linux x86_64 | ubuntu-latest の上で manylinux_2_28 のコンテナ | する (2 回) | する | 約 40 分 (手元と同じ 4 コア) |
| Linux aarch64 | ubuntu-24.04-arm (arm の機械で直に。今はクロス) + manylinux_2_28_aarch64 のコンテナ | する | しない | 約 20 分 (v3 の作りが無い) |
| Windows x64 | windows-latest (Git Bash で台本) | する (2 回) | する | 約 50〜60 分 (MSVC の LTO のリンクは遅め) |
| macOS arm64 | macos-15 | する | しない | 約 15〜20 分 |
| macOS x86_64 | macos-15-intel (Intel の機械で直に。今は arm からクロス) | する (2 回) | する | 約 50〜60 分 |

- 学習は作った wheel を動かすので、その機械で動く対象でしか PGO はできない。そのため今のクロスの 2 つ (Linux aarch64・macOS x86_64) は、ネイティブの機械に移す。
  - 移せない場合は、その job だけ `PGO=0` (LTO だけ) にする。macOS x86_64 なら、arm の機械でクロスで作り、v3 も作って dual にすることはできる。
- profile は OS・対象ごとに取り直す (profraw は対象ごとのもの)。どの job も自分で学習する。
- job は並んで動くので、全体の待ち時間は今の約 10 分から約 60 分になる見込み (一番遅い Windows・macOS Intel で決まる)。x86_64 は学習が base と v3 の 2 回になる (下の訂正)。
- Linux は、maturin の `--compatibility manylinux_2_28` が名札付けと確かめをする。2 本入りにしたあとで `auditwheel show` で両方の .so を見る。
- test の job: 今の pytest を `ENOMOTO_CORE=auto` と `ENOMOTO_CORE=base` の 2 回流す。auto では `build_cpu_level()` を表示する。GitHub の x86 の機械は v3 があるので、2 本の両方が試される。
- sdist は今のまま (ソースからの build は 1 本、LTO・PGO なし)。
- 確かめていないこと:
  - Windows (MSVC) と macOS で `-Cprofile-generate` が通ること。rustc は対応しているが、ここでは動かしていない。最初の CI の実行で見る。
  - `macos-15-intel` と `ubuntu-24.04-arm` のラベルが、このリポジトリで使えること。
  - abi3 の wheel での 2 本入り。手元では cp313 で作った。CI では `--features pyo3/abi3-py39` を付けて作るので、`_core.abi3.so` の形になる。

### 7.5 手元での確かめ (Linux x86_64)

2 本入りの wheel を pip で入れて確かめた (`var/c_dual`。base だけ・v3 だけの wheel も `var/c_base`・`var/c_v3` に入れた)。

- `pip install` は通り、`enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so` と `_core_v3.cpython-313-x86_64-linux-gnu.so` が入った。
- 読み分け: ENOMOTO_CORE が auto・空・不正な値 (警告を出して auto) では `_core_v3` (`build_cpu_level()` が "x86-64-v3")、base では `_core`、v3 では `_core_v3`。
  `model.py` が使う `_core` も読み替えた方になっている。`_core_v3` の無い wheel で v3 を指定すると、警告して base を使う。
- `tests/test_model.py` (43 件) は auto・base・v3 のどれでも全部通る。`test_version.py`・`test_select_problem.py` (9 件) も通る。

1 回目の wheel の fuzz 7 本 (`fuzz_var.sh c_dual` は auto で v3、`ENOMOTO_CORE=base fuzz_var.sh c_base`): どれも bad 0・mismatches 0。
cargo test (案 C のソース、LTO + PGO、base と v3 の設定): どちらも 420 通過・0 失敗。

#### 1 スレッドの比べ (m2)

`python run_var.py m2 base pgo pgo_v3 c_base c_v3 c_plain` (15 問 × 2 回、1 スレッド、決定的)。解析は `ana_m1.py res_m2.jsonl` → ana_m2.txt、`ana_m2b.txt`。

- base・pgo・pgo_v3 は前のソース (e2b2f0677)、c_* は案 C を入れたソース (13aada96d)。
  c_plain は案 C のソースを今の作り方 (LTO・PGO なし) で作ったもの。
- 道筋: c_base・c_v3 は c_plain と、15 問 × 2 回すべてで状態・ノード数・目的値が同じ。
  base と比べると 4 問 (mas76・rout・noswot・qiu) でノード数が違うが、c_plain も同じように違うので、作り方ではなくソースの差 (この間に入った PROBE_PAR・mps などの変更) による。目的値はどれも同じ。
- 時間の比:

| 版/基準 | 幾何平均 |
|---|---:|
| pgo/base (前のソース) | 0.918 |
| pgo_v3/base (前のソース) | 0.879 |
| c_base/c_plain (案 C のソース) | 0.884 |
| c_v3/c_plain (案 C のソース) | 0.841 |
| c_plain/base (ソースの差。4 問の道筋の違いを含む) | 1.044 |

- 道筋が base と同じ 11 問だけで、学習の中身を比べると: c_base/pgo 1.017、c_v3/pgo_v3 1.003。
  **生成問題で学習した PGO は、MIPLIB の 10 問で学習した PGO と同じだけ効く** (差は 2 回の揺れ 2.8〜4.7% の内)。
  計測の問題を学習に使わなくてよいことが確かめられた。
- c_plain は後から別に回した (交互でない) ので、c_*/c_plain は機械の揺れを含む。交互に回し直した値は 7.6 節 (m3)。
- c_v3 は v3 の profile がほぼ捨てられた作り (下の訂正)。


### 7.6 作り直した wheel (学習 2 回、hash mismatch 0)

- 作り直し: build_c2.out。ソースは c_* と同じ 13aada96d で、台本だけ直したものを作業木に写した。
  - 4 つの作り (計測用 base・base・計測用 v3・v3) のどれも hash mismatch 0。
  - 1 回目の v3 の作りは 1282。
- 1 スレッドの比べ (m3): `python run_var.py m3 c_plain c2_base c_v3 c2_v3`。4 版を交互に、15 問 × 2 回。解析は ana_m3.txt。
  - 道筋: どれも c_plain と、状態・ノード数・目的値が全部同じ。
  - 時間の比 (幾何平均):

| 版/基準 | 幾何平均 |
|---|---:|
| c2_base / c_plain (base の LTO + PGO) | 0.941 |
| c_v3 / c_plain (1 回目の v3。v3 の profile はほぼ捨てられていた) | 0.865 |
| c2_v3 / c_plain (作り直した v3。v3 の計測用の版で学習) | 0.869 |
| c2_v3 / c_v3 | 1.005 |

  同じ版の 2 回の揺れは 4.6〜5.5%。
- **profile が合うようにしても、1 スレッドの速さは変わらなかった** (c2_v3/c_v3 1.005、揺れの内)。
  捨てられていた profile の関数は、この 15 問の時間にはほとんど効いていなかったことになる。
  それでも、作りを正しく保つために学習 2 回と mismatch 0 の確かめは残す。ソースが変わると、どこに効くかが変わるため。
- 案 C の wheel は、同じソースを今の作り方 (LTO・PGO なし、1 本) で作ったものより、v3 の CPU で 13% (0.869)、v3 の無い CPU で 6% (0.941) 速い。
- 1〜6 節の pgo_v3 (前のソース、同じく profile が合っていなかった) の値も、この結果から見て、直しても大きくは変わらないと見る。

作り直した 2 本入りの wheel の確かめ:
- fuzz 7 本: `fuzz_var.sh c2_dual` (auto で v3) と `ENOMOTO_CORE=base fuzz_var.sh c2_base`。どれも bad 0・mismatches 0。
- tests/test_model.py (43 件): auto・base・v3 のどれでも全部通る。

### 7.7 入れたもの (2026-11-23、利用者の判断) と公開前の確かめ

この分岐に入れた (main・develop・PyPI には触っていない):
- .github/workflows/release.yml: release_proposed.yml の形。
  - linux・windows・macos の job は scripts/build_dist_wheel.sh で作る (学習 2 回・hash mismatch 0 の確かめ込み)。各作りのログを成果物に残す。
  - test の job は、pytest を ENOMOTO_CORE=auto と base で 2 回流す。
- README.md・README.ja.md の「インストール」と「ソースからビルドする場合」: readme_install_draft.md の内容。
  - 作る時間は約 35 分に直した。手順の 3 は学習 2 回に合わせた。

CI が実際に動くのは develop に入れたとき (push で build と test が回る) なので、公開 (main) の前に次を確かめる:

1. Windows (MSVC) で `-Cprofile-generate` の作りと学習が通り、profraw ができて llvm-profdata でまとまること。
   台本はパスを cygpath で `C:/...` に直している。
2. macOS (macos-15 の arm64、macos-15-intel の x86_64) で `-Cprofile-generate` の作りと学習が通ること。
3. `macos-15-intel` と `ubuntu-24.04-arm` のラベルが、このリポジトリで使えること (job が待ち続けないこと)。
   使えなければ、その対象は今までどおりクロスで作り、`PGO=0` にする。macOS x86_64 は `DUAL=1` のまま 2 本入りにはできる。
4. abi3 (`--features pyo3/abi3-py39`) の wheel で、2 本入りが `_core.abi3.so` と `_core_v3.abi3.so` (Windows は `_core.pyd` と `_core_v3.pyd`) になること。
   Python 3.9・3.13・3.14 の test で、auto が `x86-64-v3` を、base が `base` を読むこと。手元は cp313 の wheel でしか確かめていない。
5. 各 job のログ (build-logs-*) で、全部の作りの hash mismatch が 0 であること。台本は 0 でなければ止まる。
6. manylinux のコンテナ (manylinux_2_28) で、rustup の llvm-tools・venv・maturin の `--compatibility manylinux_2_28` が通ること。
   `auditwheel show` が 2 本の .so をどちらも manylinux_2_28 と見ること。
7. aarch64 (ubuntu-24.04-arm) で、PGO が通ること (DUAL=0、1 本)。
8. 全体の時間 (見込み 約 60 分)。GitHub の job の上限 (6 時間) には十分収まるはず。
9. 公開した wheel を、v3 の無い CPU (または ENOMOTO_CORE=base) と v3 の CPU の両方で import して解けること。test の job の 2 回で代わりに見る。

## 8. ファイル

- `build_var.sh`、`build_all.sh` (1 回目、作業木を固める前で使わない)、`build_all2.sh` (今回の全部)、`build_all.out`・`build_all2.out`
- `pgo_train.sh`、`res_pgotrain.jsonl` (学習の回)
- `apply_ffloor.py` ((a) の当て方)、`make_dual.py` ((c))。make_dual_wheel.py は scripts/ へ移した (7 節)
- `run_var.py` (1 スレッドの比べ)、`res_m1.jsonl`・`run_m1.out`、`ana_m1.py`・`ana_m1.txt`
- `fuzz_var.sh` (版ごとの fuzz 7 本)、`fuzz_lto.out`・`fuzz_pgo_v3.out`、`cargo_lto.log`・`cargo_pgo_v3.log`
- `plan_P.py`・`run_P.py`・`shards_P.json`・`res_P_s0.jsonl`・`run_P0.out`、`ci.py`・`ana_P.py` (6 節)
- var/・target_var/・pgo/ はコミットしない (.gitignore)
