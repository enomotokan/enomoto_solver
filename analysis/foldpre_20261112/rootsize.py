"""畳んだ後の前処理でどれだけ縮むか (analysis/foldpre_20261112.md): 設定ごとに 1 スレッドで解き始め、根の LP の大きさ (MIP: root LP size の行)、
前処理の最後の行・列、根の LP が解けるまでの壁時計の秒を記録して止める。
python rootsize.py <出力.jsonl> <設定,...> <問題 ...>   (wheel は FOLD_PKG)"""
import json, os, subprocess, sys, time, re
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
CF = {"nofold": "0", "def": "ri", "fP": "riP", "fQ": "riQ", "fB": "riB", "fBQ": "riBQ"}
out, cfgs, probs = sys.argv[1], sys.argv[2].split(","), sys.argv[3:]
done = set()
if os.path.exists(out):
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(out)}
for p in probs:
    for c in cfgs:
        if (p, c) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        if os.environ.get("FOLD_PKG"):
            env["PYTHONPATH"] = os.environ["FOLD_PKG"]
        env.update(ENOMOTO_MIP_FOLD=CF[c], ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="1")
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                               "--time-limit", "600", "--threads", "1"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        rec = {"problem": p, "cfg": c, "fold": [], "presolve": None, "root_lp": None, "root_size": None, "t_root": None, "t_presolve": None}
        for ln in pr.stderr:
            t = round(time.time() - t0, 2)
            if ln.startswith("MIP_FOLD:"):
                rec["fold"].append(ln.strip()[:300])
            elif ln.startswith("MIP: presolve: rows"):
                rec["presolve"] = ln.strip()[:300]; rec["t_presolve"] = t
            elif re.match(r"MIP: root LP [-0-9.e+]+ \(", ln):
                rec["root_lp"] = ln.strip()[:200]; rec["t_root"] = t
            elif ln.startswith("MIP: root LP size"):
                rec["root_size"] = ln.strip(); break
            if time.time() - t0 > 900:
                break
        pr.kill(); pr.wait()
        rec["wall"] = round(time.time() - t0, 1)
        with open(out, "a") as f:
            f.write(json.dumps(rec) + "\n")
        print(p, c, rec["root_size"], rec["t_root"], flush=True)
