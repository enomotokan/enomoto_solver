"""既定 (環境変数なし) で finish_cut の小さい係数の吸収の下限 (ENOMOTO_MIP_CUT_SMALL_FLOOR) が働く回数と、根の下界の変化を数える。
根だけ (ENOMOTO_MIP_NODE_LIMIT=1)、1 スレッド、種 1 で、下限あり (既定) と下限なし (=0) を 1 回ずつ解く。
python floorcnt.py SET 秒 [問題 ...]  (SET は P (rootgap の P の 58 問) か M (M のうち手元にある問題から 30 問))。結果は floorcnt_SET.jsonl に足す (再開できる)。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
SET, tl = sys.argv[1], sys.argv[2]
probs = sys.argv[3:]
if not probs:
    sh = json.loads((D.parent / f"shards_{SET}.json").read_text())["shards"]
    probs = sorted({s[0] for x in sh for s in x})
    if SET == "M":
        P = {s[0] for x in json.loads((D.parent / "shards_P.json").read_text())["shards"] for s in x}
        have = [p for p in probs if p not in P and (WT / ".miplib_cache" / f"{p}.mps").exists() and (WT / ".miplib_cache" / f"{p}.mps").stat().st_size < 30e6]
        probs = have[:: max(1, len(have) // 30)][:30]
OUT = D / f"floorcnt_{SET}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["floor"]))
pat = re.compile(r"cut small-coefficient floor \(all threads\): coefficients absorbed only by the floor (\d+), in cuts (\d+), cuts emptied (\d+), cuts dropped \(infinite bound\) (\d+)")
for p in probs:
    for fl in ("1", "0"):
        if (p, fl) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_CUT_SMALL_FLOOR": fl})
        t0 = time.time()
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "rc": pr.returncode}  # noqa: BLE001
        st = [list(map(int, m.groups())) for m in pat.finditer(pr.stderr)]
        st = [sum(c) for c in zip(*st)] if st else None
        rec = {"problem": p, "floor": fl, "status": res.get("status"), "bound": res.get("bound"), "root_bound": res.get("root_bound"), "objective": res.get("objective"),
               "time": res.get("time"), "nodes": res.get("nodes"), "stats": st}
        with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
        print(p, fl, rec["status"], rec["root_bound"], rec["bound"], st, round(time.time() - t0, 1), flush=True)
