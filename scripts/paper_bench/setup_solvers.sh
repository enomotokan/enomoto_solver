#!/usr/bin/env bash
# 論文用ベンチマークの比較ソルバー (CLP, SoPlex) をソースからビルドし、計時用ドライバを作る。
# HiGHS (highspy) と enomoto_solver 本体の Python 環境もここで用意する。
#
# 使い方 (リポジトリのルートで、Linux):
#     bash scripts/paper_bench/setup_solvers.sh
#
# 環境変数で上書きできるもの:
#     PREFIX          インストール先 (既定: <repo>/.paper_solvers)
#     BUILD_DIR       ビルド作業場所 (既定: $PREFIX/build)
#     CLP_VERSION     CLP のリリース (既定: 1.17.11、coinbrew の releases/<版>)
#     SOPLEX_VERSION  SoPlex のリリース (既定: 8.1.0、git タグ v<版>)
#     HIGHS_VERSION   highspy の版 (既定: 1.15.1)
#     VENV            Python 仮想環境 (既定: <repo>/.paper_venv、SKIP_PYTHON=1 で作らない)
#     JOBS            並列ビルド数 (既定: nproc)
#     PYTHON          仮想環境の元にする Python (既定: python3)
#
# 必要なもの: git, g++, make, curl, python3 (venv 付き)。cmake が無ければ仮想環境に pip で入れる。
# Rust (cargo) が無ければ rustup で入れる ($CARGO_HOME、既定 ~/.cargo。シェルの設定は変えない)。
# root 権限は要らない。
# 作るもの: $PREFIX/bin/clp_driver, $PREFIX/bin/soplex_driver, $VENV (highspy + enomoto_solver)。
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PREFIX="${PREFIX:-$REPO/.paper_solvers}"
BUILD_DIR="${BUILD_DIR:-$PREFIX/build}"
CLP_VERSION="${CLP_VERSION:-1.17.11}"
SOPLEX_VERSION="${SOPLEX_VERSION:-8.1.0}"
HIGHS_VERSION="${HIGHS_VERSION:-1.15.1}"
VENV="${VENV:-$REPO/.paper_venv}"
JOBS="${JOBS:-$(nproc)}"
PYTHON="${PYTHON:-python3}"
DRIVERS="$REPO/scripts/paper_bench/drivers"

mkdir -p "$PREFIX/bin" "$BUILD_DIR"

# ---------------------------------------------------------------- Python 環境 (cmake もここから使う)
if [[ "${SKIP_PYTHON:-0}" != "1" ]]; then
    if [[ ! -x "$VENV/bin/python" ]]; then
        # ensurepip の無い Python (python3-venv 未導入など) では pip を後から入れる。
        if ! "$PYTHON" -m venv "$VENV" >/dev/null 2>&1; then
            rm -rf "$VENV"
            "$PYTHON" -m venv --without-pip "$VENV"
            curl -fsSL https://bootstrap.pypa.io/get-pip.py | "$VENV/bin/python" - -q
        fi
    fi
    "$VENV/bin/python" -m pip install -q --upgrade pip
    "$VENV/bin/python" -m pip install -q "highspy==$HIGHS_VERSION" "maturin>=1.5,<2.0" cmake
    export PATH="$VENV/bin:$PATH"
fi
if ! command -v cmake >/dev/null; then
    echo "cmake が見つからない (SKIP_PYTHON=1 のときは自分で入れること)" >&2
    exit 1
fi

# ---------------------------------------------------------------- CLP (coinbrew)
if [[ ! -f "$PREFIX/lib/libClp.so" && ! -f "$PREFIX/lib/libClp.a" ]]; then
    mkdir -p "$BUILD_DIR/coin"
    cd "$BUILD_DIR/coin"
    if [[ ! -x coinbrew ]]; then
        curl -fsSL -o coinbrew https://raw.githubusercontent.com/coin-or/coinbrew/master/coinbrew
        chmod +x coinbrew
    fi
    ./coinbrew fetch "Clp@releases/$CLP_VERSION" --no-prompt --skip-update
    ./coinbrew build Clp --prefix="$PREFIX" --no-prompt --tests=none --parallel-jobs="$JOBS" \
        --disable-debug --without-lapack --without-blas
fi

# CLP ドライバ。pkg-config があれば使い、無ければインストール先から直接リンクする。
CLP_INC="$(find "$PREFIX/include" -name ClpSimplex.hpp -printf '%h\n' | head -1)"
COINUTILS_INC="$(find "$PREFIX/include" -name CoinPragma.hpp -printf '%h\n' | head -1)"
if command -v pkg-config >/dev/null && PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig" pkg-config --exists clp; then
    CLP_FLAGS="$(PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig" pkg-config --cflags --libs clp)"
else
    CLP_FLAGS="-I$CLP_INC -I$COINUTILS_INC -L$PREFIX/lib -lClp -lCoinUtils"
fi
# shellcheck disable=SC2086
g++ -O2 -std=c++17 -DCLP_VERSION_STR="\"$CLP_VERSION\"" "$DRIVERS/clp_driver.cpp" -o "$PREFIX/bin/clp_driver" \
    $CLP_FLAGS -Wl,-rpath,"$PREFIX/lib"

# ---------------------------------------------------------------- SoPlex (CMake)
if [[ ! -d "$PREFIX/lib/cmake/soplex" ]]; then
    rm -rf "$BUILD_DIR/soplex"
    git clone --depth 1 --branch "v$SOPLEX_VERSION" https://github.com/scipopt/soplex.git "$BUILD_DIR/soplex"
    cmake -S "$BUILD_DIR/soplex" -B "$BUILD_DIR/soplex/build" -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$PREFIX" -DGMP=off -DMPFR=off -DBOOST=off -DPAPILO=off -DZLIB=off
    cmake --build "$BUILD_DIR/soplex/build" -j "$JOBS"
    cmake --install "$BUILD_DIR/soplex/build"
fi

rm -rf "$BUILD_DIR/drivers"
cmake -S "$DRIVERS" -B "$BUILD_DIR/drivers" -DCMAKE_PREFIX_PATH="$PREFIX" -DSOPLEX_VERSION_STR="$SOPLEX_VERSION"
cmake --build "$BUILD_DIR/drivers" -j "$JOBS"
cp "$BUILD_DIR/drivers/soplex_driver" "$PREFIX/bin/soplex_driver"

# ---------------------------------------------------------------- enomoto_solver 本体
if [[ "${SKIP_PYTHON:-0}" != "1" ]]; then
    CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
    if ! command -v cargo >/dev/null && [[ ! -x "$CARGO_BIN/cargo" ]]; then
        curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
    fi
    export PATH="$CARGO_BIN:$PATH"
    cd "$REPO"
    VIRTUAL_ENV="$VENV" "$VENV/bin/maturin" develop --release
fi

echo
echo "clp_driver:    $("$PREFIX/bin/clp_driver" --version)"
echo "soplex_driver: $("$PREFIX/bin/soplex_driver" --version)"
if [[ "${SKIP_PYTHON:-0}" != "1" ]]; then
    echo "highspy:       $("$VENV/bin/python" -c 'import highspy; print(highspy.Highs().version())')"
    echo "python:        $VENV/bin/python"
fi
