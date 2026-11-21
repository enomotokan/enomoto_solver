"""最初の LP の前 (VBCLQ の e の位置) に最適値の上限があれば、pseudoobj がどれだけ固定し全体がどれだけ速くなるか (天井) の診断。
上限は診断 ENOMOTO_MIP_VBCLQ_PRE_CUTOFF=<値> で与える。前処理の中の上限だけで、木には暫定解も打ち切り値も渡らない
(pseudoobj は上限ちょうどの解を残すので、最適解は消えない。木は自分で解を探す)。最適値は scripts/miplib2017-v31.solu の =opt=。
- python ceil.py scan K NSHARD: P 58 問 + M (P と重ならない、最適値の分かる 140 問) を、上限 = 最適値で前処理の後で止め
  (ENOMOTO_MIP_DEBUG_STOP_PRESOLVED)、e の行・かけ直し後の大きさを拾う → ceil_scan.jsonl_K
- python ceil.py run SET K NSHARD [cfg,..]: SET = P (60 秒) か M (600 秒、ceil_scan で固定が 1 列以上の問題だけ)。1 スレッド・種 1。
  cfg: def (今の既定 = e)・opt (上限 = 最適値)・opt1・opt10 (最適値から最初の根の LP の値までの差の 1%・10% だけ緩めた上限。
  根の LP の値は def の行から取る) → ceil_SET.jsonl_K
どれも途中から再開できる。nice 19。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
SOL = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] == "=opt=": SOL[f[1]] = float(f[2])
def parse(out):
    r = {}
    m = re.search(r"VBCLQ e: before the LP probing, bound (\S+) \(([^)]*)\): (\d+) objective columns substituted, (\d+) fixed in (\d+) passes(.*?), slack (\S+), (\d+) columns changed", out)
    if m: r["e"] = {"bound": float(m.group(1)), "src": m.group(2), "fixed": int(m.group(4)), "infeas": "no solution better" in m.group(6), "slack": float(m.group(7)), "changed": int(m.group(8))}
    if "reaches the pseudo-objective bound" in out: r["e_opt"] = True
    if "VBCLQ e: before the LP probing: no solution" in out: r["e_nosol"] = True
    if "the presolve after the fixings says infeasible" in out: r["e_dropped"] = True
    r["vb01"] = re.findall(r"VB01 on the presolved problem: (\S+)", out)[:1]
    rows = re.findall(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", out)
    r["presolve"] = [tuple(int(v) for v in x) for x in rows[:6]]
    m = re.search(r"LP probing: root LP ([\d.]+)s \((\d+) iters", out)
    if m: r["lpprobe"] = (float(m.group(1)), int(m.group(2)))
    m = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", out)
    if m: r["root"] = (float(m.group(1)), int(m.group(2)), float(m.group(3)))
    # 最初の LP (LP による固定) の前に出た解か: ログでの順で見る
    i_lp = out.find("LP probing:")
    inc = [(m.start(), m.group(1), m.group(2), m.group(3)) for m in re.finditer(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[([^\]]*)\]", out)]
    r["inc_first"] = [(v, t, s, i_lp >= 0 and pos < i_lp) for pos, v, t, s in inc[:3]]
    early = re.findall(r"presolve: early FJ/Local-MIP.*?found (\w+)", out)
    r["fj_early"] = early[:1]
    res = [l for l in out.splitlines() if l.startswith("{")]
    r["result"] = json.loads(res[-1]) if res else None
    return r
def run1(p, env_add, tl, stop=False):
    had = (WT / ".miplib_cache" / f"{p}.mps").exists()
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SEED": "1"}); env.update(env_add)
    if stop: env["ENOMOTO_MIP_DEBUG_STOP_PRESOLVED"] = "1"
    t0 = time.time()
    try:
        pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
        out = pr.stderr + pr.stdout; to = False
    except subprocess.TimeoutExpired as e:
        out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); to = True
    r = parse(out); r["wall"] = round(time.time() - t0, 1); r["timeout"] = to
    if not had and stop: mps.unlink()
    return r
mode = sys.argv[1]
if mode == "scan":
    k, ns = int(sys.argv[2]), int(sys.argv[3])
    probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P and p in SOL]
    OUT = Path(str(D / "ceil_scan.jsonl") + f"_{k}")
    done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
    for s, p in probs[k::ns]:
        if p in done: continue
        row = {"set": s, "problem": p, "opt": SOL[p]}
        try: row.update(run1(p, {"ENOMOTO_MIP_VBCLQ_PRE_CUTOFF": repr(SOL[p])}, 120, stop=True))
        except Exception as e: row["error"] = str(e)[:200]  # noqa: BLE001
        with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
        print(json.dumps({x: row.get(x) for x in ("set", "problem", "e", "e_nosol", "wall")}), flush=True)
elif mode == "run":
    SET, k, ns = sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    cfgs = sys.argv[5].split(",") if len(sys.argv) > 5 else ["def", "opt"]
    tl = 60 if SET == "P" else 600
    if SET == "P": probs = P
    else:
        sc = [json.loads(l) for f in sorted(D.glob("ceil_scan.jsonl_*")) for l in open(f) if l.strip()]
        probs = sorted(r["problem"] for r in sc if r["set"] == "M" and (r.get("e") or {}).get("fixed", 0) > 0)
    OUT = Path(str(D / f"ceil_{SET}.jsonl") + f"_{k}")
    rows = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
    done = {(r["problem"], r["cfg"]) for r in rows}
    for p in probs[k::ns]:
        for c in cfgs:
            if (p, c) in done: continue
            env = {}
            if c == "opt": env = {"ENOMOTO_MIP_VBCLQ_PRE_CUTOFF": repr(SOL[p])}
            elif c in ("opt1", "opt10"):
                root = next((r.get("root") for r in rows if r["problem"] == p and r["cfg"] == "def" and r.get("root")), None)
                if not root: continue
                frac = 0.01 if c == "opt1" else 0.10
                # MIPLIB の MPS は最小化の形 (solu の値も)。最適値から根の LP の値までの差の frac だけ悪い側 (大きい側) に緩める
                env = {"ENOMOTO_MIP_VBCLQ_PRE_CUTOFF": repr(SOL[p] + frac * abs(SOL[p] - root[0]))}
            row = {"set": SET, "problem": p, "cfg": c, "opt": SOL[p], "env": env}
            try: row.update(run1(p, env, tl))
            except Exception as e: row["error"] = str(e)[:200]  # noqa: BLE001
            rows.append(row)
            with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
            res = row.get("result") or {}
            print(p, c, res.get("status"), res.get("time"), res.get("nodes"), (row.get("e") or {}).get("fixed"), flush=True)
