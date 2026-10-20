# 使い方: exp.sh 名前 時間 木のスレッド数 [環境変数=値 ...]  問題は PROB (既定 neos-2978193-inde)
name=$1; T=$2; th=$3; shift 3
prob=${PROB:-neos-2978193-inde}
cd /home/user/enomoto_solver/work_r2
env ENOMOTO_MIP_THREADS=4 "$@" timeout $((T+120)) python run_eno.py ../.miplib_cache/$prob.mps $T $th > $name.out 2> $name.err
python3 - "$name" <<'PY'
import sys,re
n=sys.argv[1]
err=open(n+".err",errors="replace").read().splitlines()
rows=[int(m.group(1)) for l in err if l.startswith("ENODE") for m in [re.search(r"rows=(\d+)",l)] if m]
rs=sorted(rows)
rr=f"rows med {rs[len(rs)//2]} max {rs[-1]}" if rs else ""
print(n, "|", open(n+".out").read().strip().splitlines()[-1], "|", rr)
PY
