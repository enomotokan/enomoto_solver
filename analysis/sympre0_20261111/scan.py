"""前処理の前 (DRCR の後) と後で置換の対称性を数える (2026-11-11、求解はしない)。
ENOMOTO_MIP_SYMPRE0_DIAG=<値> と ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM=1 で 1 問ずつ走らせ、`MIP: SYMPRE0 before/after` の行を読む。
問題: MIPLIB 2017 benchmark 240 問 (analysis/drcr_20261110/mip_count_ri.jsonl の順)。
使い方: python scan.py 名前 [設定,...] [並列数]  → scan_<名前>.jsonl に追記 (済んだ (問題, 設定) は飛ばす)。PROBS=a,b で問題を絞る"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {"ENOMOTO_MIP_SYMPRE0_DIAG": "1"}, "x10": {"ENOMOTO_MIP_SYMPRE0_DIAG": "x10"},
        "sassy": {"ENOMOTO_MIP_SYMPRE0_DIAG": "1", "ENOMOTO_MIP_SYMPRE": "1"}}
name = sys.argv[1]
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else ["def"]
par = int(sys.argv[3]) if len(sys.argv) > 3 else 2
probs = [json.loads(l)["name"] for l in open(WT / "analysis/drcr_20261110/mip_count_ri.jsonl")]
if os.environ.get("PROBS"): probs = os.environ["PROBS"].split(",")
OUT = D / f"scan_{name}.jsonl"
done = set()
if OUT.exists():
    for l in open(OUT):
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
TO = int(os.environ.get("SCAN_TIMEOUT", "400"))
PAT = re.compile(r"MIP: SYMPRE0 (\w+): n (\d+) m (\d+) nnz (\d+); generators (\d+), moved (\d+) columns; twins (\d+) classes (\d+) columns; orbits (\d+) on (\d+) columns \((\d+) integer\) \(([\d.]+)s")
KEYS = "n m nnz gens moved twin_classes twin_cols orbits orb_cols orb_int secs".split()

def one(p, cfg):
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        return {"problem": p, "cfg": cfg, "err": str(e)[:200]}
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1", **CFGS[cfg])
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=TO)
        err, rc = pr.stderr, pr.returncode
    except subprocess.TimeoutExpired as e:
        err, rc = (e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr or "")), "timeout"
    r = {"problem": p, "cfg": cfg, "rc": rc, "wall": round(time.time() - t0, 2)}
    for m in PAT.finditer(err):
        r[m[1]] = {k: (float(v) if k == "secs" else int(v)) for k, v in zip(KEYS, m.groups()[1:])}
    m = re.search(r"MIP: symmetry: (\d+) generators, (\d+) orbits on (\d+) columns", err)
    if m: r["log"] = [int(m[1]), int(m[2]), int(m[3])]
    m = re.search(r"MIP_FOLD: (n \d+ ->[^\n]*|no fold[^\n]*)", err)
    if m: r["fold"] = m[1][:120]
    if "symmetry: skipped" in err: r["skipped"] = True
    if "stopped after the symmetry detection" in err: r["stopped"] = True
    if "after" not in r: r["tail"] = err[-400:]
    return r

todo = [(p, c) for p in probs for c in cfgs if (p, c) not in done]
with ThreadPoolExecutor(par) as ex:
    for r in ex.map(lambda a: one(*a), todo):
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        b, a = r.get("before", {}), r.get("after", {})
        print(r["problem"], r["cfg"], "before", b.get("gens"), b.get("orb_cols"), b.get("secs"), "after", a.get("gens"), a.get("orb_cols"), a.get("secs"), r["wall"], flush=True)
