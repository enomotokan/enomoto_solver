"""型 D の確かめ: 並べ替え 0〜3 で前処理の後の大きさ・クリークの拡張・根の LP が揃うか。
python dcheck.py 秒 問題... [-e KEY=VAL ...] [-k 並べ替え数(既定 4)]
束 cand1114 + -e の環境変数で解き (1 スレッド、種 1)、根の LP の大きさの行までの道しるべを並べ替えごとに出し、違う行に印を付ける。
ログは logs_d/ に残す。"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113")); sys.path.insert(0, str(WT / "analysis/robust_20261114"))
from probs import rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
import perm as pm  # noqa: E402
args = sys.argv[1:]
TL = float(args.pop(0)); extra = {}; K = 4; probs = []
while args:
    a = args.pop(0)
    if a == "-e": k, v = args.pop(0).split("=", 1); extra[k] = v
    elif a == "-k": K = int(args.pop(0))
    else: probs.append(a)
PAT = re.compile(r"^MIP: (presolve: rows.*|root LP [-0-9.e+]+ \(\d+ iters|root LP size.*|clique extension: .*?removed|probing: \d+ of \d+ binaries probed \(\d+ passes\), \d+ fixed)")
tag = re.sub(r"[^A-Za-z0-9=_.,-]", "", "_".join(f"{k.replace('ENOMOTO_','')}={v.split('/')[-1]}" for k, v in sorted(extra.items()))) or "base"
(D / "logs_d").mkdir(exist_ok=True)
def mps_of(p, k):
    src = rb.fetch(p, WT / ".miplib_cache")
    if k == 0: return src
    out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
    if not out.exists(): out.parent.mkdir(exist_ok=True); pm.make(src, k, out)
    return out
for p in probs:
    sigs = []
    for k in range(K):
        env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
        env.update(CAND1114); env.update({kk: v.replace("{k}", str(k)) for kk, v in extra.items()}); env["ENOMOTO_MIP_THREADS"] = "1"; env["ENOMOTO_T_MIP_SEED"] = "1"; env["ENOMOTO_MIP_LOG"] = "1"
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps_of(p, k)), "--time-limit", str(TL), "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True)
        with gzip.open(D / "logs_d" / f"{p}_{tag}_p{k}.log.gz", "wt") as f: f.write(pr.stderr)
        L = []
        for l in pr.stderr.splitlines():
            m = PAT.match(l)
            if m:
                s = re.sub(r"postsolve steps \d+ ", "", m.group(1)); L.append(s)
            if l.startswith("MIP: root LP size"): break
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}
        sigs.append((L, res))
    n = max(len(s[0]) for s in sigs)
    ndiff = 0
    print(f"== {p} [{tag}]")
    for i in range(n):
        row = [s[0][i] if i < len(s[0]) else "-" for s in sigs]
        same = len(set(row)) == 1
        ndiff += not same
        print(("   " if same else " * ") + row[0][:150])
        if not same:
            for k in range(1, K): print(f"   p{k} " + row[k][:150])
    print(f"   diff lines {ndiff}; results", [(r.get('status'), round(r.get('time') or 0, 1), r.get('objective')) for _, r in sigs], flush=True)
