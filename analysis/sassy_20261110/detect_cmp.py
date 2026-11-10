"""対称性の検出だけを数え、自前の参照 (refcheck.py) と比べる (2026-11-10)。1 問を 4 つの設定 (def・pre・d1・probe) で、最初の検出の
直後に止め (ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM)、検出に渡した問題と生成元を書き出して (ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH) refcheck.py にかける。
問題: 58 問 + Mittelmann の 47 問 (shards8.json) + analysis/symsign_20261106 で生成元のあった MIPLIB の問題。同時に 1 つずつ走らせる。
使い方: python detect_cmp.py [設定,...]  → detect.jsonl に追記 (済んだ (問題, 設定) は飛ばす)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "pre": {"ENOMOTO_MIP_SYMPRE": "1"}, "d1": {"ENOMOTO_MIP_SYMPRE": "d1"}, "probe": {"ENOMOTO_MIP_SYMPRE": "probe"}}
sh = json.load(open(WT / "analysis/mitt600_20261103/shards8.json"))["problems"]
old = [json.loads(l) for f in ["detect_p58.jsonl", "detect_mitt.jsonl"] for l in open(WT / "analysis/symsign_20261106" / f)]
base = list(dict.fromkeys(PROBS + sh))
extra = sorted({r["problem"] for r in old if r["cfg"] == "def" and (r.get("gens") or 0) > 0} - set(base))
probs = base + extra
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else list(CFGS)
OUT = D / "detect.jsonl"
DUMP = Path(os.environ.get("DUMP_DIR", "/tmp/symdumps")); DUMP.mkdir(parents=True, exist_ok=True)
done = set()
if OUT.exists():
    for l in open(OUT):
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
TO = int(os.environ.get("DETECT_TIMEOUT", "400"))

def parse(err):
    r = {}
    m = re.search(r"MIP: symmetry: (\d+) generators, (\d+) orbits on (\d+) columns, (\d+) lex-leader rows \(([\d.]+)s.*?detect ([\d.]+)s", err)
    if m: r.update(gens=int(m[1]), orbits=int(m[2]), moved=int(m[3]), lex=int(m[4]), t_sym=float(m[5]), t_detect=float(m[6]))
    m = re.search(r"MIP: symmetry: none \(([\d.]+)s.*?detect ([\d.]+)s", err)
    if m: r.update(gens=0, orbits=0, moved=0, lex=0, t_sym=float(m[1]), t_detect=float(m[2]))
    if "symmetry: skipped" in err: r["skipped"] = True
    m = re.search(r"SYMPRE removed (\d+) columns and (\d+) rows of degree <= 1 \((\d+) trees at the root, ([\d.]+)s\); core (\d+) columns (\d+) rows; core generators (\d+) \(lifted (\d+), failed (\d+)\); subtree swaps (\d+) \(failed (\d+), cut (\d+)\)", err)
    if m: r["d1"] = dict(zip("rc rr roots t_red core_n core_m core_gens lifted lift_fail swaps swap_fail swap_cut".split(), [float(x) if "." in x else int(x) for x in m.groups()]))
    if "SYMPRE removed 0 vertices" in err: r["d1"] = {"rc": 0, "rr": 0}
    m = re.search(r"SYMPRE probe: (\d+) cells, (\d+) done as orbits, (\d+) probes, (\d+) generators \(([\d.]+)s(, stopped)?", err)
    if m: r["probe"] = dict(cells=int(m[1]), done=int(m[2]), probes=int(m[3]), gens=int(m[4]), secs=float(m[5]), stopped=bool(m[6]))
    m = re.search(r"refinement ([\d.]+)s, work", err)
    if m: r["t_refine"] = float(m[1])
    m = re.search(r"twins (\d+) classes .*?merged (\d+) classes", err)
    if m: r["twin_classes"] = int(m[1]); r["twin_merged"] = int(m[2])
    m = re.search(r"stopped after the symmetry detection \(ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM, ([\d.]+)s\)", err)
    if m: r["t_presolve_sym"] = float(m[1]); r["stopped"] = True
    return r

for p in probs:
    for cfg in cfgs:
        if (p, cfg) in done: continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        dump = DUMP / f"{p}.{cfg}.txt"
        if dump.exists(): dump.unlink()
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM="1", ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH=str(dump), **CFGS[cfg])
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=TO)
            err, rc = pr.stderr, pr.returncode
        except subprocess.TimeoutExpired as e:
            err, rc = (e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr or "")), "timeout"
        r = {"problem": p, "cfg": cfg, "rc": rc, "wall": round(time.time() - t0, 2), **parse(err)}
        if not r.get("stopped") and rc != "timeout": r["tail"] = err[-300:]
        if dump.exists():
            try:
                cp = subprocess.run([sys.executable, str(D / "refcheck.py"), str(dump), "120" if cfg == "def" else "15"], capture_output=True, text=True, timeout=600)
                r["ref"] = json.loads(cp.stdout.strip().splitlines()[-1])
            except Exception as e:  # noqa: BLE001
                r["ref_err"] = str(e)[:200]
            dump.unlink()
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        ref = r.get("ref", {})
        print(p, cfg, r.get("gens"), r.get("orbits"), r.get("t_detect"), "ref", ref.get("e_cols"), ref.get("ref_cols"), ref.get("wl_cols"), ref.get("missed_found"), ref.get("unknown_pairs"), ref.get("bad_gens"), r["wall"], flush=True)
