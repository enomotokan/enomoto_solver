"""既定 (環境変数なし) で finish_cut の打ち消しの確かめ (ENOMOTO_MIP_FINISH_CANCEL) が働く回数と、根の下界の変化を数える。
根だけ (ENOMOTO_MIP_NODE_LIMIT=1)、1 スレッド、種 1 で、確かめあり (既定) となし (=0) を 1 回ずつ解く (wrong_widden/floorcnt.py と同じ組み立て)。
python cancelcnt.py SET 秒 並列 [問題 ...]  (SET は P (rootgap の P の 58 問) か M (M のうち手元にある問題から 30 問))。結果は cancelcnt_SET.jsonl に足す (再開できる)。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
SET, tl, par = sys.argv[1], sys.argv[2], int(sys.argv[3])
probs = sys.argv[4:]
if not probs:
    sh = json.loads((D.parent / f"shards_{SET}.json").read_text())["shards"]
    probs = sorted({s[0] for x in sh for s in x})
    if SET == "M":
        P = {s[0] for x in json.loads((D.parent / "shards_P.json").read_text())["shards"] for s in x}
        have = [p for p in probs if p not in P and (WT / ".miplib_cache" / f"{p}.mps").exists() and (WT / ".miplib_cache" / f"{p}.mps").stat().st_size < 30e6]
        probs = have[:: max(1, len(have) // 30)][:30]
OUT = D / f"cancelcnt_{SET}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cancel"]))
pf = re.compile(r"cut small-coefficient floor \(all threads\): coefficients absorbed only by the floor (\d+), in cuts (\d+), cuts emptied (\d+), cuts dropped \(infinite bound\) (\d+)")
pc = re.compile(r"cut cancellation check \(all threads\): (?:coefficients zeroed|residual coefficients) (\d+), in cuts (\d+), of which not absorbed by the small-coefficient rule (\d+), (?:in cuts|cuts changed) (\d+), cuts emptied (\d+)")
def sm(pat, s):
    st = [list(map(int, m.groups())) for m in pat.finditer(s)]
    return [sum(c) for c in zip(*st)] if st else None
def one(job):
    p, cc = job
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_FINISH_CANCEL": cc})
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": pr.returncode}  # noqa: BLE001
    rec = {"problem": p, "cancel": cc, "status": res.get("status"), "bound": res.get("bound"), "root_bound": res.get("root_bound"), "objective": res.get("objective"),
           "time": res.get("time"), "nodes": res.get("nodes"), "floor_stats": sm(pf, pr.stderr), "cancel_stats": sm(pc, pr.stderr)}
    with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
    print(p, cc, rec["status"], rec["root_bound"], rec["bound"], rec["cancel_stats"], round(time.time() - t0, 1), flush=True)
jobs = [(p, cc) for p in probs for cc in os.environ.get("CC", "1 0").split() if (p, cc) not in done]
with ThreadPoolExecutor(par) as ex: list(ex.map(one, jobs))
