"""プロービングの呼ばれ方の表: 1 スレッド・種 1・根まで (ENOMOTO_MIP_NODE_LIMIT=1) で回し、前処理と根の再スタートの中のプロービングの回ごとに
(何回目・再スタートの後か・含意を使う形か・試した数・固定・締め付け・含意・同値な組・秒) を拾う。python survey.py TAG [ENOMOTO_X=V ...] -- 問題... >> survey.jsonl"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
i = sys.argv.index("--"); tag = sys.argv[1]; extra = sys.argv[2:i]; probs = sys.argv[i + 1:]
RX = re.compile(r"MIP: probing: (\d+) of (\d+) binaries probed \((\d+) passes\), (\d+) fixed, (\d+) bounds tightened, (\d+) implications \(([0-9.]+)s")
for p in probs:
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_NODE_LIMIT": "1"})
    for a in extra:
        k, v = a.split("=", 1); env[k] = v
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", "600", "--threads", "1"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=1500)
    err = pr.stderr
    with gzip.open(D / "logs" / f"{p}_{tag}.log.gz", "wt") as f: f.write(err)
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    seq, restart, equiv, clq = [], 0, None, False
    for l in err.splitlines():
        if re.search(r"MIP: restart(ing)? .*(presolv|restart #)", l): restart += 1
        if "CLQPRB=g" in l and "redoing" in l: clq = True
        m = re.search(r"MIP: probing: (\d+) equivalent binary pairs", l)
        if m: equiv = int(m.group(1))
        m = RX.search(l)
        if m:
            a = [int(x) for x in m.groups()[:6]] + [float(m.group(7))]
            seq.append({"restart": restart, "clq": clq, "probed": a[0], "cands": a[1], "passes": a[2], "fixed": a[3], "tight": a[4], "impl": a[5], "secs": a[6], "equiv_prev": equiv})
            clq, equiv = False, None
        if "PRBWORK" in l and "skipped" in l: seq.append({"restart": restart, "skipped": "PRBWORK"})
        if "probing skipped" in l: seq.append({"restart": restart, "skipped": l.split("(")[-1].rstrip(")")})
    pre = re.findall(r"MIP: presolve: .*?\(([0-9.]+)s\)$", err)
    tpre = re.findall(r"MIP: presolve done.*?([0-9.]+)s|presolve total ([0-9.]+)", err)
    row = {"problem": p, "tag": tag, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"), "root_bound": res.get("root_bound"), "probes": seq,
           "probe_secs": round(sum(x.get("secs", 0) for x in seq), 2)}
    with open(D / "survey.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(p, tag, row["status"], round(row["time"] or 0, 1), "probes", len(seq), "secs", row["probe_secs"], [(x.get("restart"), x.get("fixed"), x.get("tight"), x.get("impl"), x.get("secs"), x.get("skipped")) for x in seq], flush=True)
