"""試作 ENOMOTO_MIP_LU_EXTEND を既定と比べる (1 スレッド・種 1、区間計時 ENOMOTO_MIP_XPROF つき)。終わった (問題, 設定) は飛ばす。
使い方 (リポジトリの根から): python analysis/lpiter_20261123/cmp11.py <秒>   出力: res11.jsonl、ログ logs/c11_<設定>_<問題>.log.gz"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "300"
probs = [("fhnw-binpack4-4", "ENOMOTO_MIP_NODE_CLEANUP=q", "q"), ("fhnw-binpack4-4", "", ""), ("bnatt400", "", ""), ("neos-873061", "", ""),
         ("binkar10_1", "", ""), ("neos-911970", "", ""), ("mik-250-20-75-4", "", ""), ("neos5", "", ""), ("mas76", "", ""), ("pk1", "", ""),
         ("misc07", "", ""), ("nw04", "", "")]
cfgs = [("def", {}), ("ext", {"ENOMOTO_MIP_LU_EXTEND": "1"})]
out = D / "res11.jsonl"
done = set()
if out.exists():
    for l in open(out):
        r = json.loads(l); done.add((r["problem"], r["base"], r["cfg"]))
for p, base, btag in probs:
    for cname, cenv in cfgs:
        if (p, btag, cname) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(cenv)
        if base:
            k, v = base.split("="); env[k] = v
        env.update({"ENOMOTO_MIP_XPROF": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
        r = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps", "--time-limit", tl, "--threads", "1"],
                           cwd=R, env=env, capture_output=True, text=True)
        log = r.stdout + r.stderr
        with gzip.open(D / "logs" / f"c11_{cname}{btag}_{p}.log.gz", "wt") as f:
            f.write(log)
        res = next((json.loads(l) for l in reversed(log.splitlines()) if l.startswith("{")), None)
        xp = {m.group(1): float(m.group(2)) for m in re.finditer(r"^XPROF (\S+)\s+([\d.]+) ms", log, re.M)}
        row = {"problem": p, "base": btag, "cfg": cname, "result": res, "w_lu_ms": xp.get("w_lu"), "main_ms": xp.get("main"),
               "lu_add_del_ms": sum(xp.get(k, 0.0) for k in ("lp_add_rows", "lp_del_rows", "lp_del_add_rows")),
               "counts": {k: v for k, v in xp.items() if k.startswith("n_")}}
        open(out, "a").write(json.dumps(row) + "\n")
        rr = res or {}
        print(p, btag, cname, rr.get("status"), rr.get("objective"), rr.get("nodes"), "%.1f" % (rr.get("time") or 0), "w_lu", row["w_lu_ms"], "adddel %.0f" % row["lu_add_del_ms"], row["counts"], flush=True)
