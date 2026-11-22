"""rootpar: 根 (前処理〜木に入るまで) の時間の内訳と、段ごとに実際に動いているスレッドの数。python run.py PLAN (例 plan.json、記録済みは飛ばす)。
計画の 1 項目は [問題, 種, スレッド数]。ENOMOTO_MIP_LOG=1 で解き (既定の動きは変えない)、stderr の行を受け取った時刻 (起動からの壁の秒) つきで読み、
/proc/PID/task/*/stat を 0.2 秒ごとに見て、段ごとの CPU 秒を出す (プロセス全体の CPU 秒の差。終わったスレッドの分も入る。スレッドごとの差は最も多い 1 本と、区間の半分以上動いた本数にだけ使う)。res.jsonl に 1 回 1 行。
段の区切り (印の行、求解ごと = 再スタートごとに繰り返す):
  pre  : 求解の始め (起動 / 前の求解の `MIP: restart`) 〜 `MIP: solving the root LP`  (外側の前処理・この求解の前処理・プロービング・対称性・FJ の起動など)
  lp   : 〜 `MIP: root LP <値> (` (根の LP)
  cut  : 〜 最後の `MIP: cut round` (カットのループ。分離・LP の解き直し・カットの選び・OBJPROBE)
  heur : 〜 `MIP: after root heuristics` (根のヒューリスティクス)
  post : 〜 次の求解の始め (`MIP: restart`) か `TRSTAT: tree start` (再スタートの判定・根の後始末)
  tree : `TRSTAT: tree start` 〜 終わり
印がない段は 0 秒 (次の印までをまとめる)。根のヒューリスティクスの名前ごとの秒 (`MIP: root heuristic NAME: ... (d)`) も足す。"""
import json, os, re, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PLAN = json.loads((D / sys.argv[1]).read_text())
OUT = D / "res.jsonl"
TL = 120
TICK = os.sysconf("SC_CLK_TCK")
M_RESTART = re.compile(r"^MIP: restart \d+ \(")
M_SOLVE = re.compile(r"^MIP: solving the root LP")
M_LP = re.compile(r"^MIP: root LP [-0-9.eE+]+ \(")
M_CUT = re.compile(r"^MIP: cut round \d+:")
M_HEUR = re.compile(r"^MIP: after root heuristics")
M_TREE = re.compile(r"^TRSTAT: tree start")
M_RH = re.compile(r"^MIP: root heuristic (.+?): (\S+).*\(([0-9.]+)s\)$")
def cpu_snap(pid):
    """スレッドごとの CPU 秒と、"__proc__" にプロセス全体 (終わったスレッドの分も含む、/proc/PID/stat) の CPU 秒"""
    out = {}
    try:
        s = open(f"/proc/{pid}/stat").read(); f = s[s.rindex(")") + 2:].split()
        out["__proc__"] = (int(f[11]) + int(f[12])) / TICK
    except (OSError, ValueError):
        return {}
    try:
        for t in os.listdir(f"/proc/{pid}/task"):
            try:
                s = open(f"/proc/{pid}/task/{t}/stat").read(); f = s[s.rindex(")") + 2:].split()
                out[t] = (int(f[11]) + int(f[12])) / TICK
            except (OSError, ValueError):
                pass
    except OSError:
        pass
    return out
def one(p, seed, th):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(TL), "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
    lines = []
    def rd():
        for l in pr.stderr: lines.append((time.time() - t0, l.rstrip("\n")))
    th_ = threading.Thread(target=rd, daemon=True); th_.start()
    snaps = []
    while pr.poll() is None:
        sn = cpu_snap(pr.pid)
        if sn: snaps.append((time.time() - t0, sn))  # 終わりかけで読めなかった回は捨てる
        time.sleep(0.2)
        if time.time() - t0 > TL + 300: pr.kill()
    wall = time.time() - t0
    th_.join(5)
    out = pr.stdout.read()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    # 段の区切り
    marks = []  # (時刻, 段の名前 (この印で終わる段))
    seg = 0; heur = {}
    for t, l in lines:
        if M_RESTART.match(l): marks.append((t, "post")); seg += 1
        elif M_SOLVE.match(l): marks.append((t, "pre"))
        elif M_LP.match(l): marks.append((t, "lp"))
        elif M_CUT.match(l): marks.append((t, "cut"))
        elif M_HEUR.match(l): marks.append((t, "heur"))
        elif M_TREE.match(l): marks.append((t, "post")); marks.append((t, "TREE"))
        m = M_RH.match(l)
        if m: heur[m.group(1)] = round(heur.get(m.group(1), 0.0) + float(m.group(3)), 3)
    # 同じ段の印が続く (cut round が何回も) ときは最後のものを段の終わりにする
    ivs = []; prev = 0.0; i = 0
    while i < len(marks):
        t, name = marks[i]
        if name == "TREE": break
        while i + 1 < len(marks) and marks[i + 1][1] == name and name == "cut": i += 1; t = marks[i][0]
        ivs.append((name, prev, t)); prev = t; i += 1
    tree_t = next((t for t, n in marks if n == "TREE"), None)
    if tree_t is not None: ivs.append(("tree", tree_t, wall))
    else: ivs.append(("rest", prev, wall))
    # 段ごとの CPU (スレッドごとの差分)
    def at(t):
        best = None
        for s in snaps:
            if s[0] <= t: best = s
            else: break
        return best[1] if best else {}
    phases = {}
    for name, a, b in ivs:
        ca, cb = at(a), at(b)
        d = sorted((cb.get(k, 0) - ca.get(k, 0) for k in cb if k != "__proc__"), reverse=True)
        ph = phases.setdefault(name, {"secs": 0.0, "cpu": 0.0, "cpu_top": 0.0, "busy_threads_secs": 0.0})
        # cpu はプロセス全体の差 (区間の中で始まって終わったスレッドの分も入る)。cpu_top・busy は区間の両端で生きていたスレッドだけ
        ph["secs"] += b - a; ph["cpu"] += cb.get("__proc__", 0) - ca.get("__proc__", 0); ph["cpu_top"] += d[0] if d else 0.0
        ph["busy_threads_secs"] += sum(1 for x in d if x > 0.5 * (b - a)) * (b - a)  # 区間の半分以上動いたスレッドの数 × 秒
    for ph in phases.values():
        for k in ph: ph[k] = round(ph[k], 3)
    return {"result": res, "wall": round(wall, 2), "segments": seg + 1, "phases": phases, "root_heur": heur,
            "tree_start_wall": None if tree_t is None else round(tree_t, 2)}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["seed"], r["threads"]))
for p, seed, th in PLAN:
    if (p, seed, th) in done: continue
    r = one(p, seed, th)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "seed": seed, "threads": th, **r, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, seed, th, r["result"].get("status"), r["result"].get("time"), {k: (v["secs"], round(v["cpu"] / max(v["secs"], 1e-9), 2)) for k, v in r["phases"].items()}, flush=True)
