#!/usr/bin/env bash
# 配る wheel を作る: LTO (fat、codegen-units 1) + PGO。x86_64 ではさらに x86-64-v3 向けの _core を同梱する (dual)。
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
# rustc に渡すパスは Windows では C:/... の形にする
native() { if command -v cygpath > /dev/null 2>&1; then cygpath -m "$1"; else echo "$1"; fi; }
venv_py() { if [ -x "$1/bin/python" ]; then echo "$1/bin/python"; else echo "$1/Scripts/python.exe"; fi; }
now() { date +%s; }
rm -rf "$WORK"; mkdir -p "$WORK" "$OUT"
W=$(cd "$WORK" && pwd)
T0=$(now)

# build 名前 追加のRUSTFLAGS → $W/名前/ に wheel 1 本
build() {
  local name=$1 flags=$2 t
  t=$(now)
  echo "== build $name: RUSTFLAGS=$BASE_RUSTFLAGS $flags"
  RUSTFLAGS="$BASE_RUSTFLAGS $flags" CARGO_TARGET_DIR="$ROOT/target/dist-$name" \
    maturin build --release $MATURIN_ARGS --out "$W/$name"
  echo "== build $name: $(( $(now) - t )) 秒"
}

USE=""
if [ "$PGO" = 1 ]; then
  SYSROOT=$(rustc --print sysroot)
  HOST=$(rustc -vV | sed -n 's/^host: //p')
  PROFDATA="$SYSROOT/lib/rustlib/$HOST/bin/llvm-profdata"
  [ -x "$PROFDATA" ] || PROFDATA="$PROFDATA.exe"
  [ -x "$PROFDATA" ] || { echo "llvm-profdata が無い: rustup component add llvm-tools" >&2; exit 1; }
  mkdir -p "$W/pgo-raw"
  build gen "-Cprofile-generate=$(native "$W/pgo-raw")"
  "$PY" -m venv "$W/venv"
  VPY=$(venv_py "$W/venv")
  "$VPY" -m pip install -q --no-index --no-deps "$W"/gen/*.whl
  t=$(now)
  echo "== train: scripts/pgo_train.py $TRAIN_ARGS"
  (cd "$W" && "$VPY" "$ROOT/scripts/pgo_train.py" $TRAIN_ARGS 2> "$W/train.err")
  echo "== train: $(( $(now) - t )) 秒"
  "$PROFDATA" merge -o "$W/merged.profdata" "$W"/pgo-raw/*.profraw
  ls -l "$W/merged.profdata"
  USE="-Cprofile-use=$(native "$W/merged.profdata")"
fi

build base "$USE"
if [ "$DUAL" = 1 ]; then
  build v3 "$USE -Ctarget-cpu=x86-64-v3"
  "$PY" "$ROOT/scripts/make_dual_wheel.py" "$W"/base/*.whl "$W"/v3/*.whl "$OUT"
else
  cp "$W"/base/*.whl "$OUT"/
fi
ls -l "$OUT"
echo "== 全体: $(( $(now) - T0 )) 秒"
