"""試作 ENOMOTO_MIP_LU_EXTEND (木の中だけで使う形) を既定と比べる (種 1、区間計時 ENOMOTO_MIP_XPROF つき)。終わった (問題, スレッド, 設定) は飛ばす。
使い方 (リポジトリの根から): python analysis/luxroot_20261124/cmp.py <秒>   出力: res.jsonl、ログ logs/<設定>_t<スレッド>_<問題>.log.gz"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "300"
P1 = ["nw04", "neos-827175", "air03", "binkar10_1", "neos-911970", "mik-250-20-75-4", "neos5", "mas76", "pk1", "misc07", "fhnw-binpack4-4"]
runs = [(p, 1) for p in P1] + [("neos-827175", 4), ("nw04", 4)]
cfgs = [("def", {}), ("ext", {"ENOMOTO_MIP_LU_EXTEND": "1"})]
out = D / "res.jsonl"
done = set()
if out.exists():
    for l in open(out):
        r = json.loads(l); done.add((r["problem"], r["threads"], r["cfg"]))
for p, th in runs:
    for cname, cenv in cfgs:
        if (p, th, cname) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(cenv)
        env.update({"ENOMOTO_MIP_XPROF": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1"})
        r = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps", "--time-limit", tl, "--threads", str(th)],
                           cwd=R, env=env, capture_output=True, text=True)
        log = r.stdout + r.stderr
        with gzip.open(D / "logs" / f"{cname}_t{th}_{p}.log.gz", "wt") as f:
            f.write(log)
        res = next((json.loads(l) for l in reversed(log.splitlines()) if l.startswith("{")), None)
        xp = {m.group(1): float(m.group(2)) for m in re.finditer(r"^XPROF (\S+)\s+([\d.]+) ms", log, re.M)}
        rounds = len(re.findall(r"^MIP: cut round \d+: \d+ cuts", log, re.M))
        row = {"problem": p, "threads": th, "cfg": cname, "result": res, "root_cut_rounds": rounds, "w_lu_ms": xp.get("w_lu"),
               "counts": {k: v for k, v in xp.items() if k.startswith("n_")}}
        open(out, "a").write(json.dumps(row) + "\n")
        rr = res or {}
        print(p, th, cname, rr.get("status"), rr.get("objective"), rr.get("nodes"), "%.1f" % (rr.get("time") or 0), "rounds", rounds, row["counts"], flush=True)
