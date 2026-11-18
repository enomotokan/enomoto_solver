"""適用範囲の確認 (README 6 節): MIPLIB 2017 benchmark 240 問 (Mittelmann 186 問を含む) を前処理だけ (ENOMOTO_MIP_DEBUG_PRETIME=stop) で
def・spls の 2 回ずつ回し、疎化の計数 (`MIP: sparsify-late stats`) と前処理の後の大きさ (`PRETIME ... m n nnz`) を scan.jsonl に。
python scan.py [並列数 (既定 2)]。済んだ (問題, 設定) は飛ばす。もとから .miplib_cache になかった MPS は回した後に消す (ディスクの節約)。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "spls": {"ENOMOTO_MIP_SPARSIFY_LATE": "s"}}
probs = [json.loads(l)["name"] for l in open(WT / "analysis/drcr_20261110/mip_count_ri.jsonl")]
mitt = {p for v in json.load(open(WT / "analysis/cbcimpl_20261029/mitt_parts.json")) for p in v}
par = int(sys.argv[1]) if len(sys.argv) > 1 else 2
OUT = D / "scan.jsonl"
done = set()
if OUT.exists():
    for l in open(OUT):
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
ST = re.compile(r"sparsify-late stats m (\d+) n (\d+) bincols (\d+) changed (\d+) binonly (\d+) shrunk<=1 (\d+) nnz (\d+) -> (\d+) gate ([\d.]+)")
PT = re.compile(r"PRETIME total=([\d.]+) m=(\d+) n=(\d+) nnz=(\d+)")
def one(p):
    cache = WT / ".miplib_cache"; had = (cache / f"{p}.mps").exists()
    try: mps = rb.fetch(p, cache)
    except Exception as e:  # noqa: BLE001
        return [{"problem": p, "cfg": c, "err": str(e)[:200]} for c in CFGS]
    rows = []
    for c, kv in CFGS.items():
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="2", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_DEBUG_PRETIME="stop", **kv)
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "2"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=600)
            err = pr.stderr
        except subprocess.TimeoutExpired:
            err = "TIMEOUT"
        st = ST.findall(err); pt = PT.findall(err)
        rows.append({"problem": p, "cfg": c, "mitt": p in mitt, "wall": round(time.time() - t0, 1), "stats": [list(s) for s in st],
                     "pre": list(pt[-1]) if pt else None, "timeout": err == "TIMEOUT"})
    if not had: mps.unlink(missing_ok=True)
    return rows
todo = [p for p in probs if any((p, c) not in done for c in CFGS)]
with ThreadPoolExecutor(par) as ex:
    for rows in ex.map(one, todo):
        with open(OUT, "a") as f:
            for r in rows: f.write(json.dumps(r) + "\n")
        print(rows[0]["problem"] if rows else "", [(r["cfg"], r.get("pre"), r.get("stats")) for r in rows], flush=True)
