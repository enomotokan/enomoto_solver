# 使い方: exp.sh 名前 時間 木のスレッド数 [環境変数=値 ...]  (打ち切り値なしにするには CUTOFF=none)
name=$1; T=$2; th=$3; shift 3
cd /home/user/enomoto_solver/work_tree
env ENOMOTO_MIP_THREADS=4 "$@" timeout $((T+120)) python run_eno.py ../.miplib_cache/neos-2978193-inde.mps $T $th > $name.out 2> $name.err
python - "$name" <<'PY'
import sys,re
n=sys.argv[1]
lines=[l for l in open(n+".out",errors="replace") if re.match(r"\s+[\d.]+s\s+\d+",l)]
last=lines[-1].split() if lines else []
nodes=sum(1 for l in open(n+".err",errors="replace") if l.startswith(("ENODE","EBR")))
print(n, "last:", " ".join(last), "| logged nodes", nodes, "|", open(n+".out").read().strip().splitlines()[-1])
PY
