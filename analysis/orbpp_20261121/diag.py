"""ORBPP の診断の 1 回 (1 節)。diag.patch を当てたビルドで、`ENOMOTO_MIP_ORBDIAG=1`・`ENOMOTO_MIP_LOG=1` を付けて当方を回し、
ログを logs/PROB_sSEED_TAG.log.gz に、要約を diag.jsonl に足す。
  python diag.py PROB SEED TAG THREADS TL [ENOMOTO_X=V ...]
  python diag.py run PLAN.json        計画 ([[prob, seed, tag, threads, tl, [args...]], ...]) を順に。記録済み (prob, seed, tag, threads) は飛ばす"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
OUT = D / "diag.jsonl"


def summarize(err):
    s = {}
    m = re.search(r"ORBPP (\d+) of (\d+) orbitopes static.*fixed (\d+) columns", err)
    if m: s["orbpp"] = [int(x) for x in m.groups()]
    m = re.findall(r"ORBPP_REPROBE: .*fixed by propagation (\d+), by the orbitopes (\d+), by probing (\d+), free binaries (\d+) -> (\d+)", err)
    if m: s["reprobe"] = [int(x) for x in m[0]]
    m = re.findall(r"MIP: orbitopes (\d+): fixed (\d+) bounds, pruned (\d+) nodes", err)
    if m: s["orb_tree"] = [int(x) for x in m[-1]]
    m = re.findall(r"MIP: orbital fixing \((\d+) generators\): fixed (\d+) bounds at (\d+) nodes, pruned (\d+) nodes", err)
    if m: s["of_tree"] = [int(x) for x in m[-1]]
    s["tree_orb"] = [[int(a), int(b), int(c)] for a, b, c in re.findall(r"ORBDIAG tree orbitope (\d+) \S+ static \w+: fixed (\d+) pruned (\d+)", err)]
    s["rootlp"] = [[t, int(k), float(tot), float(up), float(um), int(fr)] for t, k, tot, up, um, fr in
                   re.findall(r"ORBDIAG rootlp (r\d+) orbitope (\d+) \S+ static \w+ packing_rows \d+: total (\S+) upper (\S+) upper_min_perm (\S+) fractional (\d+)", err)]
    s["restarts"] = len(re.findall(r"MIP: restart(?:ing)? .*(?:presolv|restart #)", err))
    m = re.findall(r"TRSTAT: tree start .* restarts (\d+) lb (\S+) ub (\S+)", err)
    if m: s["tree_start"] = [int(m[-1][0]), float(m[-1][1]), float(m[-1][2])]
    s["incumbents"] = len(re.findall(r"MIP: new incumbent", err))
    m = re.findall(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[([^\]]*)\]", err)
    s["inc_src"] = {}
    for z, n, t, src in m: s["inc_src"][src] = s["inc_src"].get(src, 0) + 1
    if m: s["last_inc"] = [float(m[-1][0]), int(m[-1][1]), float(m[-1][2]), m[-1][3]]
    return s


def one(prob, seed, tag, threads, tl, args):
    import run_miplib_benchmark as rb
    mps = rb.fetch(prob, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_ORBDIAG": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": str(threads), "ENOMOTO_T_MIP_SEED": str(seed)})
    for a in args:
        k, v = a.split("=", 1); env[k] = v
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(threads)],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 600)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    res.pop("stderr", None)
    with gzip.open(D / "logs" / f"{prob}_s{seed}_t{threads}_{tag}.log.gz", "wt") as f: f.write(pr.stderr)
    row = {"problem": prob, "seed": seed, "tag": tag, "threads": threads, "tl": tl, "args": args, "result": res, "diag": summarize(pr.stderr),
           "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(prob, seed, tag, threads, res.get("status"), res.get("nodes"), round(res.get("time") or 0, 1), res.get("objective"), flush=True)


if __name__ == "__main__":
    if sys.argv[1] == "run":
        plan = json.loads(Path(sys.argv[2]).read_text())
        done = set()
        if OUT.exists():
            for l in OUT.read_text().splitlines():
                r = json.loads(l); done.add((r["problem"], r["seed"], r["tag"], r["threads"]))
        for prob, seed, tag, th, tl, args in plan:
            if (prob, seed, tag, th) not in done: one(prob, seed, tag, th, tl, args)
    else:
        one(sys.argv[1], int(sys.argv[2]), sys.argv[3], int(sys.argv[4]), float(sys.argv[5]), sys.argv[6:])
