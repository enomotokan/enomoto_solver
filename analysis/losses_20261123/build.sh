#!/bin/sh
# 負けている問題の測り直し用の当方の wheel (PGO + x86-64-v3、LTO fat・codegen-units 1) を作る。analysis/buildopt_20261122 の pgo_v3 と同じ作り方
# (build_var.sh・pgo_train.sh を 1 本にまとめたもの)。Cargo.toml は変えず、環境変数だけで作り方を替える。
#
#   sh analysis/losses_20261123/build.sh            # 学習から: pgogen を作る → 学習 10 問 → merged.profdata → pgo_v3 の wheel
#   PROF=analysis/losses_20261123/pgo/merged.profdata.gz sh analysis/losses_20261123/build.sh   # コミット済みの学習結果で pgo_v3 だけ作る
#     (同じコミット・同じ rustc 1.97.0 なら .so は同じものになる: 2 回作って so_sha1 が一致した。wheel の sha1 は zip の時刻で変わる)
#
# 出力: analysis/losses_20261123/dist/enomoto_solver-*.whl と dist/build.txt (コミット・道具の版・wheel と .so の sha1)。
# 作るのはこのリポジトリの今のコミット (作業ツリーの変更は含めない: git worktree で今の HEAD を別に取り出して作る)。
# 学習の問題は、測る 48 問 (picked.json) と重ならない P・M の 10 問を 1 スレッド・種 1・上限 30 秒で解く。
# 要るもの: rustup の stable (rustc 1.97.0 で確かめた)、`rustup component add llvm-tools` (llvm-profdata)、maturin、highspy (学習の MPS の読み込み)。
set -e
D=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$D/../.." && pwd)
PY=${PYTHON:-python}
COMMIT=${COMMIT:-$(git -C "$REPO" rev-parse --short HEAD)}  # COMMIT=… で作るコミットを選べる (既定 HEAD)
# 作業木と学習結果は固定の場所に置く (2026-11-23 の直し): 場所や -Cprofile-use のパスが RUSTFLAGS・-C metadata に入り、学習の版と計測の版で
# 食い違うのを避ける。前の版は mktemp の毎回違う場所に作業木を作り、学習用を x86-64-v3 なしで作っていたので、計測の版の作り直しで
# 当方のクレートの関数 1281 個の学習結果が「hash mismatch」で捨てられていた (README 3 節)
B=/tmp/enomoto_losses_build
WT=$B/src
git -C "$REPO" worktree remove --force "$WT" >/dev/null 2>&1 || true
rm -rf "$WT"; mkdir -p "$B"
git -C "$REPO" worktree add --detach "$WT" "$COMMIT" >/dev/null 2>&1
trap 'git -C "$REPO" worktree remove --force "$WT" >/dev/null 2>&1 || true' EXIT
TRAIN=neos17,binkar10_1,eil33-2,neos-827175,nu25-pr12,dcmulti,qiu,rout,mas76,neos-911970
LTO="CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1"
PD=$(ls ~/.rustup/toolchains/*/lib/rustlib/*/bin/llvm-profdata 2>/dev/null | head -n 1)
build() {  # $1 = 名前 (target の場所), $2 = RUSTFLAGS
  T="$D/target/$1"; rm -rf "$T/wheels"
  (cd "$WT" && env $LTO CARGO_TARGET_DIR="$T" RUSTFLAGS="$2" maturin build --release > "$T.log" 2>&1 || true; grep -E "^error|Built wheel" "$T.log" >&2 || true)
  ls -t "$T"/wheels/enomoto_solver-*.whl | head -n 1
}
if [ -z "$PROF" ]; then
  [ -n "$PD" ] || { echo "llvm-profdata がない: rustup component add llvm-tools"; exit 1; }
  echo "== 1. 学習用 (pgogen) を作る ($COMMIT)"
  rm -rf "$D/pgo/raw"; mkdir -p "$D/pgo/raw"
  # 学習用も計測用と同じ target-cpu で作る (違うと関数の形が変わり、学習結果が当たらない)
  W=$(build pgogen "-C target-cpu=x86-64-v3 -Cprofile-generate=$D/pgo/raw")
  G="$D/target/pgogen_py"; rm -rf "$G"; mkdir -p "$G/enomoto_solver"
  cp "$WT"/python/enomoto_solver/*.py "$G/enomoto_solver/"
  $PY -c "import zipfile,sys;z=zipfile.ZipFile(sys.argv[1]);[open(sys.argv[2]+'/'+n.split('/')[-1],'wb').write(z.read(n)) for n in z.namelist() if n.endswith(('.so','.pyd'))]" "$W" "$G/enomoto_solver"
  echo "== 2. 学習 ($TRAIN、1 スレッド・上限 30 秒)"
  for p in $(echo $TRAIN | tr , ' '); do
    MPS=$($PY -c "import sys;sys.path.insert(0,'$REPO/scripts');import run_miplib_benchmark as rb;from pathlib import Path;print(rb.fetch('$p',Path('$REPO/.miplib_cache')))")
    env -u ENOMOTO_MIP_VB01 PYTHONPATH="$G" ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 $PY "$REPO/scripts/run_miplib_benchmark.py" --worker-ours "$MPS" --time-limit 30 --threads 1 2>/dev/null | tail -n 1 | cut -c1-100
  done
  "$PD" merge -o "$D/pgo/merged.profdata" "$D"/pgo/raw/*.profraw
  gzip -9 -k -f "$D/pgo/merged.profdata"   # コミットするのは .gz (2.5 MB)
  PROF="$D/pgo/merged.profdata"
fi
PROF=$(cd "$(dirname "$PROF")" && pwd)/$(basename "$PROF")
case $PROF in *.gz) gzip -dc "$PROF" > "$B/merged.profdata" ;; *) cp "$PROF" "$B/merged.profdata" ;; esac
PROF=$B/merged.profdata
echo "== 3. pgo_v3 を作る (学習結果 $PROF)"
W=$(build pgo_v3 "-C target-cpu=x86-64-v3 -Cprofile-use=$PROF")
mkdir -p "$D/dist"; rm -f "$D"/dist/*.whl; cp "$W" "$D/dist/"
WD="$D/dist/$(basename "$W")"
MISMATCH=$(grep -c "hash mismatch" "$D/target/pgo_v3.log" || true)
SO_SHA=$($PY -c "import zipfile,hashlib,sys;z=zipfile.ZipFile(sys.argv[1]);print([hashlib.sha1(z.read(n)).hexdigest() for n in z.namelist() if n.endswith(('.so','.pyd'))][0])" "$WD")
{ echo "commit $COMMIT"; echo "rustc $(rustc --version)"; echo "maturin $(maturin --version)"; echo "profdata $PROF sha1 $(sha1sum "$PROF" | cut -c1-40)"
  echo "env LTO=fat CGU=1 RUSTFLAGS=-C target-cpu=x86-64-v3 -Cprofile-use"; echo "wheel $(basename "$WD") sha1 $(sha1sum "$WD" | cut -c1-40)"; echo "so_sha1 $SO_SHA"
  echo "pgo_hash_mismatch $MISMATCH (学習結果を捨てた関数の数。0 に近いはず)"; } > "$D/dist/build.txt"
cat "$D/dist/build.txt"
echo "入れ方: pip install --force-reinstall --no-deps $WD"
