"""PGO が当たっているかの時間の確かめ: python pgo_check.py 名前=wheel ... → res_pgocheck.jsonl と表。
wheel を target/pgocheck/<名前>/ に展開し、PYTHONPATH で読ませて、1 スレッド・種 1 (決定的) で学習と重ならない 6 問を 2 回ずつ解く (問題ごとに版の順を回す)。
道筋 (状態・ノード数・目的値) が版で同じことと、時間の比 (版 / 最初の版、2 回のうち速い方どうし) を出す。"""
import hashlib, json, math, os, subprocess, sys, zipfile
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["air03", "neos8", "exp-1-500-5-5", "qnet1", "mik-250-20-75-4", "markshare_4_0"]
vers = [a.split("=", 1) for a in sys.argv[1:]]
dirs = {}
for name, whl in vers:
    d = D / "target" / "pgocheck" / name
    if not d.exists():
        zipfile.ZipFile(whl).extractall(d)
    dirs[name] = d
out = D / "res_pgocheck.jsonl"
R = {}
for rep in range(2):
    for i, p in enumerate(PROBS):
        order = vers[(i + rep) % len(vers):] + vers[:(i + rep) % len(vers)]
        for name, _ in order:
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_") and k != "PYTHONPATH"}
            env.update({"PYTHONPATH": str(dirs[name]), "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "200", "--threads", "1"],
                               cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            R.setdefault((p, name), []).append(res)
            with open(out, "a") as f: f.write(json.dumps({"problem": p, "ver": name, "rep": rep, "result": res}) + "\n")
            print(rep, p, name, res["status"], res["nodes"], f"{res['time']:.2f}", flush=True)
base = vers[0][0]
print("\n| 問題 | " + " | ".join(f"{n} (秒)" for n, _ in vers) + " | " + " | ".join(f"{n}/{base}" for n, _ in vers[1:]) + " | 道筋 |")
logs = {n: [] for n, _ in vers[1:]}
for p in PROBS:
    tb = {n: min(x["time"] for x in R[(p, n)]) for n, _ in vers}
    same = len({(x["status"], x["nodes"], round(x["objective"] or 0, 6)) for n, _ in vers for x in R[(p, n)]}) == 1
    for n, _ in vers[1:]: logs[n].append(math.log(tb[n] / tb[base]))
    print(f"| {p} | " + " | ".join(f"{tb[n]:.2f}" for n, _ in vers) + " | " + " | ".join(f"{tb[n]/tb[base]:.3f}" for n, _ in vers[1:]) + f" | {'同じ' if same else '違う'} |")
print("幾何平均:", {n: round(math.exp(sum(v) / len(v)), 3) for n, v in logs.items()})
