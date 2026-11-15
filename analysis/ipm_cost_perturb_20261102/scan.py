"""Netlib の各問題を ipm_crossover で解き、クロスオーバーの段ごとの数を 1 行の JSON にする。
使い方: scan.py δ 出力.jsonl [問題名 ...]"""
import json, os, re, subprocess, sys
REPO = "/home/user/enomoto_solver"
d, out = sys.argv[1], sys.argv[2]
names = sys.argv[3:] or open(f"{REPO}/.netlib_cache/problems.txt").read().split()
done = set()
if os.path.exists(out):
    done = {json.loads(l)["p"] for l in open(out)}
env = dict(os.environ, ENOMOTO_DEBUG_CROSSOVER="1", ENOMOTO_T_IPM_COST_PERTURB=d)
for p in names:
    if p in done:
        continue
    try:
        r = subprocess.run([f"{REPO}/.venv/bin/python", f"{REPO}/scripts/crossover_bench/run.py", "--worker",
                            f"{REPO}/.netlib_cache/mps/{p}.mps", "ipm_crossover"], capture_output=True, text=True,
                           timeout=float(os.environ.get("TL", "300")), env=env)
        txt = r.stderr + r.stdout
    except subprocess.TimeoutExpired:
        txt = ""
    rec = {"p": p}
    def g(pat, key, f=int):
        m = re.findall(pat, txt)
        if m:
            rec[key] = f(m[-1])
    g(r"CROSSOVER ipm status=\w+ iters=(\d+)", "ipm_it")
    g(r"CROSSOVER ipm .* t=([\d.]+)s", "ipm_t", float)
    g(r"CROSSOVER m=(\d+)", "m")
    g(r"basic_after_detect=(\d+)", "det")
    g(r"primal_push steps=(\d+)", "pp")
    g(r"primal_push steps=\d+ basic=(\d+)", "after_pp")
    g(r"dual_push steps=(\d+)", "dp")
    g(r"leftover superbasics=(\d+)", "sb")
    g(r"vertex obj=\S+ L\(y_ipm\)=\S+ rel gap=(\S+)", "gap", float)
    rec["accepted"] = "cleanup status" not in txt and "vertex obj" in txt
    g(r"dual_infeas=(\d+)", "dinf")
    m = re.search(r"RESULT (\{.*\})", txt)
    if m:
        res = json.loads(m.group(1))
        rec.update(t=res["time"], st=res["status"], obj=res["obj"])
    else:
        rec.update(st="timeout_or_crash")
    rec["n_xo"] = txt.count("CROSSOVER ipm status")
    with open(out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec), flush=True)
