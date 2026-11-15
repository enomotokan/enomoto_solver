"""根の LP が ROOTLP_SCALE の条件に当たるかを数える。ENOMOTO_MIP_ROOTLP_SCALE=1 にしつつ閾値を大きくして働かせず (=既定と同じ動き)、
`ROOTLP_SCALE:` の行 (行数・係数の幅) だけを出す。1 スレッド・短い上限で 4 問ずつ並べて回す。
使い方: python scan.py 上限秒 並列数 問題..."""
import json, os, subprocess, sys, re
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
D = Path(__file__).resolve().parent
WT = D.parents[1]
tl, par = sys.argv[1], int(sys.argv[2])
probs = sys.argv[3:]
done = {json.loads(l)["problem"] for l in open(D / "scan.jsonl")} if (D / "scan.jsonl").exists() else set()
(D / "scanlogs").mkdir(exist_ok=True)
def one(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_ROOTLP_SCALE="1", ENOMOTO_SIMPLEX_SCALE_LOG="1",
               ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M="1000000000")
    log = D / "scanlogs" / f"{p}.log"
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                             "--time-limit", tl, "--threads", "1"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=open(log, "w"), text=True, timeout=float(tl) + 240)
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "err": str(e)[:200]}
    txt = log.read_text(errors="replace")
    ev = []
    for l in txt.splitlines():
        m = re.match(r"ROOTLP_SCALE: rows (\d+) cols (\d+) range (\S+) warm (\w+)", l)
        if m: ev.append(dict(rows=int(m[1]), cols=int(m[2]), range=float(m[3]), warm=m[4] == "true"))
    rl = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", txt)
    r = dict(problem=p, events=ev, root_lp=(float(rl[3]) if rl else None), root_iters=(int(rl[2]) if rl else None), status=res.get("status"), time=res.get("time"))
    with open(D / "scan.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
    print(r, flush=True)
with ThreadPoolExecutor(par) as ex:
    list(ex.map(one, [p for p in probs if p not in done]))
