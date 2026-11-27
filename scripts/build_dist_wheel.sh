#!/usr/bin/env bash
# 配る wheel を作る: LTO (fat、codegen-units 1) + PGO。x86_64 ではさらに x86-64-v3 向けの _core を同梱する (dual)。
# PGO の学習は版ごと (base は base の、v3 は v3 の計測用の版で) に行う。
# 手元の `maturin develop`・`maturin build` と Cargo.toml の既定は変えない (環境変数でだけ変える)。
# 測った結果と設計は analysis/buildopt_20261122/README.md。
#
#   bash scripts/build_dist_wheel.sh            → dist/ に wheel 1 本
#
# 環境変数 (どれも省略できる):
#   PY            学習に使う Python (既定 python3。Windows の Git Bash では python)
#   MATURIN_ARGS  maturin build に足す引数 (例: "--features pyo3/abi3-py39 --compatibility manylinux_2_28")
#   OUT           できた wheel を置く所 (既定 dist)
#   WORK          途中のものを置く所 (既定 target/dist-work)
#   PGO           1 なら PGO する (既定 1)。0 なら LTO だけ (学習を回せない、クロスで作る対象など)
#   DUAL          1 なら x86-64-v3 版も作って同梱する。既定は x86_64 の機械なら 1、それ以外は 0
#   TRAIN_ARGS    scripts/pgo_train.py に渡す引数 (既定 "--time-limit 10 --threads 1,4")
#   MAX_MISMATCH  PGO の profile と合わない関数の数の上限 (既定 0。超えたら止める)
# 道具: maturin、rustup の llvm-tools (PGO のとき。`rustup component add llvm-tools`)。
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
PY=${PY:-python3}
MATURIN_ARGS=${MATURIN_ARGS:-}
OUT=${OUT:-dist}
WORK=${WORK:-target/dist-work}
PGO=${PGO:-1}
TRAIN_ARGS=${TRAIN_ARGS:---time-limit 10 --threads 1,4}
case "$(uname -m)" in x86_64|amd64|AMD64) DUAL=${DUAL:-1} ;; *) DUAL=${DUAL:-0} ;; esac
BASE_RUSTFLAGS=${RUSTFLAGS:-}
export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
# Windows の既定の文字コード (cp1252) では日本語の表示で落ちるので、Python の入出力を UTF-8 にする
export PYTHONUTF8=1 PYTHONIOENCODING=utf-8
# rustc に渡すパスは Windows では C:/... の形にする
native() { if command -v cygpath > /dev/null 2>&1; then cygpath -m "$1"; else echo "$1"; fi; }
venv_py() { if [ -x "$1/bin/python" ]; then echo "$1/bin/python"; else echo "$1/Scripts/python.exe"; fi; }
now() { date +%s; }
rm -rf "$WORK"; mkdir -p "$WORK" "$OUT"
W=$(cd "$WORK" && pwd)
T0=$(now)

# build 名前 追加のRUSTFLAGS → $W/名前/ に wheel 1 本、ログは $W/名前.log。
# PGO の profile と合わない関数 ("function control flow change detected (hash mismatch)") の数を数え、
# MAX_MISMATCH (既定 0) を超えたら止める (学習した版と target-cpu が違うと、関数の形が変わって profile が捨てられる)。
build() {
  local name=$1 flags=$2 t n
  t=$(now)
  echo "== build $name: RUSTFLAGS=$BASE_RUSTFLAGS $flags"
  RUSTFLAGS="$BASE_RUSTFLAGS $flags" CARGO_TARGET_DIR="$ROOT/target/dist-$name" \
    maturin build --release $MATURIN_ARGS --out "$W/$name" > "$W/$name.log" 2>&1 || { tail -50 "$W/$name.log"; exit 1; }
  grep -E "Built wheel" "$W/$name.log" || true
  n=$(grep -c "hash mismatch" "$W/$name.log" || true)
  echo "== build $name: $(( $(now) - t )) 秒、hash mismatch $n"
  if [ "$n" -gt "${MAX_MISMATCH:-0}" ]; then
    grep "hash mismatch" "$W/$name.log" | head -5
    echo "PGO の profile と合わない関数が $n 個 (MAX_MISMATCH=${MAX_MISMATCH:-0})" >&2; exit 1
  fi
}

# train 名前 target-cpu の RUSTFLAGS → 計測用の版を作って学習し、$W/名前.profdata を作る。
# base と v3 は命令が違うので関数の形 (CFG) も違い、profile は同じ target-cpu の計測用の版で取ったものしか合わない。
# そのため学習は版ごとに行う (x86_64 の dual では 2 回)。
train() {
  local name=$1 cpu=$2 t vpy
  mkdir -p "$W/raw-$name"
  build "gen-$name" "-Cprofile-generate=$(native "$W/raw-$name") $cpu"
  "$PY" -m venv "$W/venv-$name"
  vpy=$(venv_py "$W/venv-$name")
  "$vpy" -m pip install -q --no-index --no-deps "$W"/gen-$name/*.whl
  if [ -n "$cpu" ]; then  # v3 の計測用の版を動かすので、この機械が v3 に対応している必要がある
    (cd "$W" && "$vpy" -c "import enomoto_solver._core as c, sys; sys.exit(0 if c.cpu_x86_64_v3() else 'この機械は x86-64-v3 に対応していないので v3 の学習ができない (PGO=0 か DUAL=0 で作る)')")
  fi
  t=$(now)
  echo "== train $name: scripts/pgo_train.py $TRAIN_ARGS"
  # 落ちたときは原因が CI のログに残るよう、エラー出力の末尾を出してから止める (-X faulthandler で落ちた所の Python の呼び出しも出す)
  (cd "$W" && "$vpy" -X faulthandler "$ROOT/scripts/pgo_train.py" $TRAIN_ARGS > "$W/train-$name.out" 2> "$W/train-$name.err") || {
    rc=$?; echo "== train $name: 終了コード $rc"; echo "-- train-$name.out の末尾"; tail -20 "$W/train-$name.out"
    echo "-- train-$name.err の末尾"; tail -60 "$W/train-$name.err"; exit 1; }
  tail -1 "$W/train-$name.out"
  echo "== train $name: $(( $(now) - t )) 秒"
  "$PROFDATA" merge -o "$W/$name.profdata" "$W"/raw-$name/*.profraw
}

V3CPU="-Ctarget-cpu=x86-64-v3"
if [ "$PGO" = 1 ]; then
  SYSROOT=$(rustc --print sysroot)
  HOST=$(rustc -vV | sed -n 's/^host: //p')
  PROFDATA="$SYSROOT/lib/rustlib/$HOST/bin/llvm-profdata"
  [ -x "$PROFDATA" ] || PROFDATA="$PROFDATA.exe"
  [ -x "$PROFDATA" ] || { echo "llvm-profdata が無い: rustup component add llvm-tools" >&2; exit 1; }
  train base ""
  build base "-Cprofile-use=$(native "$W/base.profdata")"
  if [ "$DUAL" = 1 ]; then
    train v3 "$V3CPU"
    build v3 "-Cprofile-use=$(native "$W/v3.profdata") $V3CPU"
  fi
else
  build base ""
  [ "$DUAL" = 1 ] && build v3 "$V3CPU"
fi
if [ "$DUAL" = 1 ]; then
  "$PY" "$ROOT/scripts/make_dual_wheel.py" "$W"/base/*.whl "$W"/v3/*.whl "$OUT"
else
  cp "$W"/base/*.whl "$OUT"/
fi
ls -l "$OUT"
echo "== 全体: $(( $(now) - T0 )) 秒"
