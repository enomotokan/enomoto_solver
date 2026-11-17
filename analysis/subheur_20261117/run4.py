"""サブ MIP の中の解探し (analysis/subheur_20261117): 本体の問題を 既定 と 試作 (ENOMOTO_MIP_SUBMIP_HEUR) で 4 スレッド・600 秒、1 問ずつ順に解く。
python run4.py 形,... 問題,... [種]   (形: base = 既定、それ以外は ENOMOTO_MIP_SUBMIP_HEUR の値)
ログは logs/<形>_<問題>_s<種>.log (各行に経過秒)、結果は res4.jsonl (暫定解の更新の時刻と値も)。"""
import json, os, re, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
forms, probs = sys.argv[1].split(","), sys.argv[2].split(",")
seed = int(sys.argv[3]) if len(sys.argv) > 3 else 1
TL = float(os.environ.get("TL", "600"))
for p in probs:
    for form in forms:
        lf = D / "logs" / f"{form}_{p}_s{seed}.log"
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
        if form != "base":
            env["ENOMOTO_MIP_SUBMIP_HEUR"] = form
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                               "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
        out = []
        th = threading.Thread(target=lambda: out.append(pr.stdout.read())); th.start()
        incs = []
        with open(lf, "w") as f:
            for line in pr.stderr:
                el = time.time() - t0
                f.write(f"[{el:8.2f}] {line}")
                m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]", line)
                if m:
                    incs.append((round(el, 1), float(m.group(1)), m.group(4)))
        pr.wait(); th.join()
        try:
            r = json.loads([l for l in out[0].splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            r = {"status": "crash"}
        row = {"problem": p, "form": form, "seed": seed, "at": time.strftime("%H:%M:%S")} | {k: r.get(k) for k in ("status", "objective", "bound", "nodes", "time")}
        row["incumbents"] = incs[:40]
        with open(D / "res4.jsonl", "a") as f:
            f.write(json.dumps(row) + "\n")
        print({k: row[k] for k in ("problem", "form", "status", "objective", "bound", "time")}, "first", incs[:1], flush=True)
