#!/bin/sh
# ビルド設定の違う版を作る (Cargo.toml は変えず、環境変数で profile・RUSTFLAGS を替える)。
# sh build_var.sh NAME → var/NAME/enomoto_solver/ (python/enomoto_solver の .py の写しと、その版の _core*.so)、var/NAME/build.txt (時間・wheel の大きさ・sha1)。
# 計測では PYTHONPATH=var/NAME を前に置いて、その版を読ませる。target は版ごとに analysis/buildopt_20261122/target_var/NAME (互いに作り直しにならないように)。
# NAME: base (今の既定) / lto (lto = "fat"・codegen-units = 1) / v2・v3 (base + target-cpu=x86-64-v2・v3) / lto_v2・lto_v3
#       pgogen (lto + -Cprofile-generate) / pgo (lto + -Cprofile-use=pgo/merged.profdata) / pgo_v3
set -e
N=$1; D=$(cd "$(dirname "$0")" && pwd); R=$(cd "$D/../.." && pwd)
unset CARGO_PROFILE_RELEASE_LTO CARGO_PROFILE_RELEASE_CODEGEN_UNITS RUSTFLAGS
case $N in
  base) ;;
  lto) export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 ;;
  v2|v3) export RUSTFLAGS="-C target-cpu=x86-64-$N" ;;
  lto_v2|lto_v3) export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 RUSTFLAGS="-C target-cpu=x86-64-${N#lto_}" ;;
  pgogen) export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 RUSTFLAGS="-Cprofile-generate=$D/pgo/raw" ;;
  pgo) export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 RUSTFLAGS="-Cprofile-use=$D/pgo/merged.profdata" ;;
  pgo_v3) export CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 RUSTFLAGS="-C target-cpu=x86-64-v3 -Cprofile-use=$D/pgo/merged.profdata" ;;
  *) echo "unknown $N"; exit 1 ;;
esac
export CARGO_TARGET_DIR="$D/target_var/$N"
cd "$R"
rm -rf "$CARGO_TARGET_DIR/wheels"
t0=$(date +%s)
maturin build --release 2>&1 | grep -E "^(error|warning: .*profile)|Built wheel" || true
t1=$(date +%s)
W=$(ls -t "$CARGO_TARGET_DIR"/wheels/enomoto_solver-*.whl | head -n 1)
O="$D/var/$N"; rm -rf "$O"; mkdir -p "$O/enomoto_solver"
cp python/enomoto_solver/*.py "$O/enomoto_solver/"
python -c "import zipfile,sys;z=zipfile.ZipFile(sys.argv[1]);[open(sys.argv[2]+'/'+n.split('/')[-1],'wb').write(z.read(n)) for n in z.namelist() if n.endswith('.so')]" "$W" "$O/enomoto_solver"
SO=$(ls "$O"/enomoto_solver/_core*.so)
{ echo "name $N"; echo "commit $(git rev-parse --short HEAD)"; echo "env LTO=${CARGO_PROFILE_RELEASE_LTO:-} CGU=${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-} RUSTFLAGS=${RUSTFLAGS:-}"
  echo "build_secs $((t1 - t0))"; echo "wheel_bytes $(stat -c %s "$W")"; echo "so_bytes $(stat -c %s "$SO")"; echo "so_sha1 $(sha1sum "$SO" | cut -c1-40)"; } > "$O/build.txt"
cat "$O/build.txt"
