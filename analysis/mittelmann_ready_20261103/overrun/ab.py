"""使い方: python ab.py 接頭辞 回数 設定 問題...
今の既定 (off: ENOMOTO_MIP_OVERRUN3 なし) と試作 (on: ENOMOTO_MIP_OVERRUN3=設定) を交互に、gdb なしで 1 つずつ回す (4 スレッド・60 秒、
ENOMOTO_MIP_LOG=1)。logs/接頭辞.jsonl に {problem, cfg, rep, result, wall, cpu} を 1 行ずつ書く (wall は Python の起動・MPS の読み込みを含む
壁時計、cpu は子のユーザ + システムの CPU 秒)。ログは logs/接頭辞_設定_問題_回.log"""
import json, os, resource, subprocess, sys, time
from pathlib import Path

WT = Path(__file__).resolve().parents[3]
O = Path(__file__).resolve().parent / "logs"
tag, n, on = sys.argv[1], int(sys.argv[2]), sys.argv[3]
tl = os.environ.get("AB_TIME_LIMIT", "60")  # 上限を変えるとき
cfgs = os.environ.get("AB_CFGS", "off,on").split(",")  # 片方だけ回すとき (例 AB_CFGS=off)
for r in range(n):
    for p in sys.argv[4:]:
        for c in cfgs:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4")
            if c == "on":
                env["ENOMOTO_MIP_OVERRUN3"] = on
            ru0 = resource.getrusage(resource.RUSAGE_CHILDREN)
            t0 = time.perf_counter()
            pr = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps", "--time-limit", tl, "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 2 + 200)
            wall = time.perf_counter() - t0
            ru1 = resource.getrusage(resource.RUSAGE_CHILDREN)
            cpu = (ru1.ru_utime - ru0.ru_utime) + (ru1.ru_stime - ru0.ru_stime)
            (O / f"{tag}_{c}_{p}_{r}.log").write_text(pr.stderr + pr.stdout)
            js = [l for l in pr.stdout.splitlines() if l.startswith("{")]
            res = json.loads(js[-1]) if js else {"status": f"crash rc={pr.returncode}"}
            with open(O / f"{tag}.jsonl", "a") as f:
                # maxrss: これまでの子の最大の常駐メモリ (KB、Linux)。1 回ごとに別の子なので、大きくなった回のその値
                f.write(json.dumps({"problem": p, "cfg": c, "rep": r, "result": res, "wall": wall, "cpu": cpu, "maxrss_kb": ru1.ru_maxrss}) + "\n")
