"""parspeed (i): 木のスレッドあたりのノード処理が 4 スレッドで落ちる理由の内訳。python run_thr.py PLAN (例 plan_thr.json、記録済みは飛ばす)。
計画の 1 項目は [問題, 種, 形]。形は t1 (1 スレッド 1 本)、t4 (4 スレッド 1 本)、c4 (同じ 1 スレッドの求解を別々のプロセスで 4 本同時)。
どれも ENOMOTO_MIP_LOG=1・ENOMOTO_PROF_TREE_PAR=1 (既定の動きは変えない診断) で 120 秒。res_thr.jsonl に 1 本 1 行 (c4 は 4 行、copy 0〜3)。
行に残すもの: 結果、根と木の分け (run.py と同じ split)、プロセスの CPU 時間 (os.wait4 の user・sys)、スレッドごとの CPU 秒
(/proc/PID/task/*/stat を 0.5 秒ごとに見た最後の値、多い順)、stderr のうち TREEPAR・HEURROLE・TRSTAT・`MIP: tree search threads`・
最後の進み具合の行 (`MIP:  ...s nodes`)。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PLAN = json.loads((D / sys.argv[1]).read_text())
OUT = D / "res_thr.jsonl"
TL = 120
TICK = os.sysconf("SC_CLK_TCK")
TR = re.compile(r"^TRSTAT: tree start outer ([0-9.eE+-]+)")
FIN = re.compile(r"^MIP: finish start \S+ \(([0-9.eE+-]+)s\)")
KEEP = re.compile(r"^(TREEPAR|HEURROLE|TRSTAT|MIP: tree search threads)")
PROG = re.compile(r"^MIP: +[0-9.]+s nodes")
def split_of(lines):
    tr = [float(m.group(1)) for m in map(TR.match, lines) if m]
    fin = [float(m.group(1)) for m in map(FIN.match, lines) if m]
    return {"tree_started": bool(tr), "root_secs": tr[-1] if tr else None, "restarts": max(len(tr) - 1, 0), "finish_secs": fin[-1] if fin else None}
def task_cpu(pid, acc):
    try:
        for t in os.listdir(f"/proc/{pid}/task"):
            try:
                s = open(f"/proc/{pid}/task/{t}/stat").read()
                name = s[s.index("(") + 1:s.rindex(")")]; f = s[s.rindex(")") + 2:].split()
                acc[t] = (name, (int(f[11]) + int(f[12])) / TICK)
            except (OSError, ValueError):
                pass
    except OSError:
        pass
def start(p, seed, th):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1", "ENOMOTO_PROF_TREE_PAR": "1"})
    errf = tempfile.TemporaryFile(mode="w+", encoding="utf-8", errors="replace")
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(TL), "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=errf, text=True)
    return {"pr": pr, "err": errf, "t0": time.time(), "acc": {}, "ru": None, "wall": None}
def run_group(p, seed, th, copies):
    js = [start(p, seed, th) for _ in range(copies)]
    while any(j["ru"] is None for j in js):
        for j in js:
            if j["ru"] is not None: continue
            task_cpu(j["pr"].pid, j["acc"])
            pid, st, ru = os.wait4(j["pr"].pid, os.WNOHANG)
            if pid:
                j["ru"] = ru; j["wall"] = round(time.time() - j["t0"], 2)
            elif time.time() - j["t0"] > TL + 300:
                j["pr"].kill()
        time.sleep(0.5)
    rows = []
    for i, j in enumerate(js):
        out = j["pr"].stdout.read()
        try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        j["err"].seek(0); lines = j["err"].read().splitlines(); j["err"].close()
        prog = [l for l in lines if PROG.match(l)]
        thr = sorted(((round(c, 2), n) for n, c in j["acc"].values()), reverse=True)
        rows.append({"copy": i, "result": res, "split": split_of(lines), "wall": j["wall"], "cpu_user": round(j["ru"].ru_utime, 2), "cpu_sys": round(j["ru"].ru_stime, 2),
                     "threads_cpu": [c for c, n in thr], "threads_n": len(thr), "log": [l for l in lines if KEEP.match(l)] + prog[-1:]})
    return rows
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["seed"], r["mode"]))
for p, seed, mode in PLAN:
    if (p, seed, mode) in done: continue
    th, copies = {"t1": (1, 1), "t4": (4, 1), "c4": (1, 4)}[mode]
    rows = run_group(p, seed, th, copies)
    with open(OUT, "a") as f:
        for r in rows:
            f.write(json.dumps({"problem": p, "seed": seed, "mode": mode, **r, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, seed, mode, [(r["result"].get("status"), r["result"].get("time"), r["result"].get("nodes")) for r in rows], flush=True)
