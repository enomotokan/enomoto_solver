"""深さ 3 の RENS・深さ 2 の RENS (LP before cuts) を止める試作の比べ (2026-11-01)。
4 スレッド・60 秒、ソルバーは同時に 1 つだけ。ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1 (`SMT ...` の行を出す)。
使い方: python run.py [回数] [形,...] [問題,...]。ログは logs/<問題>_<形>_<回>.log.gz。回は問題ごとに形を回す順で並べる
(形の間の揺れを時刻でそろえるため、1 回目を全部の形で回してから 2 回目)。"""
import gzip, os, subprocess, sys, time
from pathlib import Path

D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
L = D / "logs"
PROBS = ["drayage-100-23", "physiciansched6-2", "mas76", "blp-ar98", "dcmulti",  # 深さ 2 の RENS が最終解を出した 5 問
         "n5-3", "neos-1582420", "neos-2978193-inde",  # 損の問題
         "h80x6320d", "qiu", "mc11"]  # 得の問題
# 追加の 2 問 (python run.py 3 def,d2,lp1,d2lp1 supportcase26,neos-1456979): 前回の 58 問で深さ 2 の RENS (LP before cuts) の
# 16.4 秒のうち 14.8 秒が supportcase26、深さ 3 の RENS の 4.4 秒のうち 3.3 秒が neos-1456979 (analysis/submip_where_20261101/logs)
EXTRA = ["supportcase26", "neos-1456979"]
CFGS = {
    "def": {},
    "d2": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "2"},
    "lp1": {"ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
    "d2lp1": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
}

if __name__ == "__main__":
    reps = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else list(CFGS)
    probs = sys.argv[3].split(",") if len(sys.argv) > 3 else PROBS
    L.mkdir(exist_ok=True)
    for r in range(reps):
        for p in probs:
            for c in cfgs:
                out = L / f"{p}_{c}_{r}.log.gz"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SUBMIP_VERBOSE="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                t0 = time.time()
                res = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                      "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                with gzip.open(out, "wt") as f:
                    f.write(f"# wall={time.time()-t0:.1f} cfg={c} env={CFGS[c]}\n" + res.stderr + "\n" + res.stdout)
                last = res.stdout.strip().splitlines()[-1][:160] if res.stdout.strip() else "NO OUTPUT"
                print(out.name, f"{time.time()-t0:.1f}s", last, flush=True)
