"""前処理の結果が既定・PREPAR・スレッドの数で同じかを確かめる (analysis/prepar_20261107.md)。
ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3 で対称性の行を足した後の問題を書き出し (ENOMOTO_MIP_DEBUG_STOP_PRESOLVED で止める)、
MPS の md5 と、ログの数 (問題の大きさ・プロービングの固定・含意・LP による固定) を並べる。
使い方: python determ.py OUT.jsonl TL 問題,... 設定,... (設定は "def:4" や "pl:2" = PREPAR の値:スレッド数)"""
import hashlib, json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT, TL, probs, cfgs = Path(sys.argv[1]), sys.argv[2], sys.argv[3].split(","), sys.argv[4].split(",")
tmp = Path(tempfile.mkdtemp())
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for cfg in cfgs:
        val, th = cfg.split(":")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        f = tmp / f"{p}_{val}_{th}.mps"
        env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_DEBUG_PRETIME": "1",
                    "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(f), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
        # DETERM_NORACE=1: 基底のない LP の同時実行 (勝ち負けが時間で決まる) を止める (ENOMOTO_MIP_MC11FIX1_RACE=s を全部の大きさに)
        if os.environ.get("DETERM_NORACE"):
            env.update({"ENOMOTO_MIP_MC11FIX1_RACE": "s", "ENOMOTO_T_MIP_MC11FIX1_RACE_SIZE": "1e15"})
        if val != "def":
            env["ENOMOTO_MIP_PREPAR"] = val
        t0 = time.time()
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", th],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=float(TL) + 300)
        err = pr.stderr
        md5 = hashlib.md5(f.read_bytes()).hexdigest()[:12] if f.exists() else None
        if f.exists():
            f.unlink()
        r = {"problem": p, "cfg": cfg, "norace": bool(os.environ.get("DETERM_NORACE")), "wall": round(time.time() - t0, 2), "md5": md5}
        m = re.search(r"PRETIME total=([\d.]+) m=(\d+) n=(\d+) nnz=(\d+) free=(\d+)", err)
        pt = [l for l in err.splitlines() if l.startswith("PRETIME")]
        if pt:
            r["pretime"] = pt[-1]
        fin = re.search(r"presolve finished in ([\d.]+)s", err)
        r["pre_secs"] = float(fin.group(1)) if fin else None
        r["probing"] = [l.split("MIP: probing: ")[1] for l in err.splitlines() if "MIP: probing: " in l and ("binaries probed" in l or "PREPAR" in l or "equivalent binary pairs" in l)]
        r["lpprobe"] = [l.split("MIP: LP probing: ")[1] for l in err.splitlines() if "MIP: LP probing: tried" in l]
        r["presolve"] = [l for l in err.splitlines() if l.startswith("Presolve:")]
        r["sym"] = [l.split("MIP: symmetry: ")[1][:80] for l in err.splitlines() if "MIP: symmetry: " in l][-1:]
        with OUT.open("a") as fo:
            fo.write(json.dumps(r, ensure_ascii=False) + "\n")
        print(p, cfg, r["wall"], md5, flush=True)
