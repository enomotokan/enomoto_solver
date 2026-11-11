"""(1) 先に確かめること: 0-1 変数が 9 割以上の問題 (kinds.json) を既定 (環境変数なし) で ENOMOTO_MIP_LOG=1 で解き、暫定解の更新の
時刻・出どころと、解き終わった時刻を res_t.jsonl に書く。58 問の該当は 60 秒・種 1〜2、res8 の該当 (58 問にないもの) は 600 秒・種 1。
ログは logs/<問題>_<秒>_s<種>.log。途中から再開できる。使い方: python run_t.py"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
K = json.loads((D / "kinds.json").read_text())
sel = [p for p, v in K.items() if v["bin"] >= 0.9 * v["n"]]
tasks = [(p, 60, s) for s in (1, 2) for p in sel if K[p]["in58"]] + [(p, 600, 1) for p in sel if K[p]["inres8"] and not K[p]["in58"]]
tasks += [(p, 600, 1) for p in sel if K[p]["inres8"] and K[p]["in58"]]
out = D / "res_t.jsonl"
done = {(r["problem"], r["tl"], r["seed"]) for r in map(json.loads, out.read_text().splitlines())} if out.exists() else set()
RE = re.compile(r"MIP: new incumbent (\S+) .*\(([0-9.]+)s\)(?: \[(.*)\])?")
for p, tl, s in tasks:
    if (p, tl, s) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(s)})
    log = D / "logs" / f"{p}_{tl}_s{s}.log"
    with open(log, "w") as ferr:
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl),
                                 "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ferr, text=True, timeout=tl + 300)
            stdout = pr.stdout
        except subprocess.TimeoutExpired:
            stdout = ""
    lines = log.read_text(errors="replace").splitlines()
    incs = [(float(m.group(1)), float(m.group(2)), m.group(3)) for m in map(RE.search, lines) if m]
    fin = [float(x) for l in lines for x in re.findall(r"MIP: solve returned \S+ after ([0-9.]+)s", l)]
    try:
        res = json.loads([l for l in stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash"}
    r = {"problem": p, "tl": tl, "seed": s, "status": res.get("status"), "time": res.get("time"), "obj": res.get("objective"),
         "bound": res.get("bound"), "nodes": res.get("nodes"), "first_t": res.get("first_solution_time"),
         "log_end_t": fin[-1] if fin else None, "incs": incs}
    with out.open("a") as f:
        f.write(json.dumps(r) + "\n")
    print(p, tl, s, r["status"], r["time"], len(incs), incs[-1] if incs else None, flush=True)
