"""rootpar_ph の診断: 4 スレッドの根の前処理 (pre) とヒューリスティクス (heur) の段の中で、どの処理が主のスレッドだけで (ほかのスレッドが空いたまま) 動いているか。
python diag.py PLAN OUT  (PLAN は [問題, 種, スレッド数] の並び、OUT は 1 回 1 行の jsonl、記録済みは飛ばす)。
analysis/rootpar_20261122/run.py の形 (ENOMOTO_MIP_LOG=1 の stderr の行を受け取った時刻と、0.1 秒ごとの /proc のプロセス全体の CPU 秒) を使い、
最初の `TRSTAT: tree start` (木に入った時点) で止める (根だけを測る)。
- 段: rootpar と同じ印 (pre: 〜`solving the root LP`、lp、cut: 〜最後の `cut round`、heur: 〜`after root heuristics`、post)。
- 段の中の処理: 続く 2 行の間の区間を、区間の終わりの行の「名前」に割り当てる (処理は終わりに 1 行出すので)。名前は行の先頭から数字・括弧を除いた
  最初の 5 語。区間ごとに壁の秒とプロセス全体の CPU 秒を足す。CPU/秒 が 1 に近い処理は主のスレッドだけで動いている (ほかの 3 本が空いている)。
- 空いているスレッドの秒 = Σ 区間の (スレッド数 × 壁の秒 − CPU 秒)。"""
import json, os, re, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
PLAN = json.loads((D / sys.argv[1]).read_text())
OUT = D / sys.argv[2]
TL = float(os.environ.get("DIAG_TL", "120"))
EXTRA = dict(a.split("=", 1) for a in os.environ.get("DIAG_EXTRA", "").split() if a)
TICK = os.sysconf("SC_CLK_TCK")
M_RESTART = re.compile(r"^MIP: restart \d+ \(")
M_SOLVE = re.compile(r"^MIP: solving the root LP")
M_LP = re.compile(r"^MIP: root LP [-0-9.eE+]+ \(")
M_CUT = re.compile(r"^MIP: cut round \d+:")
M_HEUR = re.compile(r"^MIP: after root heuristics")
M_TREE = re.compile(r"^TRSTAT: tree start")
def name_of(l):
    l = re.sub(r"\([^)]*\)", "", l)
    l = re.sub(r"[-+]?[0-9][0-9.eE+-]*", "#", l)
    w = l.replace(":", " :").split()
    return " ".join(w[:6])
def cpu(pid):
    try:
        s = open(f"/proc/{pid}/stat").read(); f = s[s.rindex(")") + 2:].split()
        return (int(f[11]) + int(f[12])) / TICK
    except (OSError, ValueError):
        return None
def one(p, seed, th):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(EXTRA)
    env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(TL), "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
    lines = []; stop = threading.Event()
    def rd():
        for l in pr.stderr:
            lines.append((time.time() - t0, l.rstrip("\n")))
            if M_TREE.match(l): stop.set()
    th_ = threading.Thread(target=rd, daemon=True); th_.start()
    snaps = []
    while pr.poll() is None:
        c = cpu(pr.pid)
        if c is not None: snaps.append((time.time() - t0, c))
        if stop.is_set() or time.time() - t0 > TL + 120:
            pr.kill(); break
        time.sleep(0.1)
    pr.wait(); th_.join(5)
    def at(t):
        best = 0.0
        for s in snaps:
            if s[0] <= t: best = s[1]
            else: break
        return best
    phase = "pre"; out_ph = {}; steps = {}; prev_t = 0.0; first_solve = None
    for t, l in lines:
        nm = name_of(l)
        d_w = t - prev_t; d_c = at(t) - at(prev_t)
        key = f"{phase}|{nm}"
        s = steps.setdefault(key, [0.0, 0.0, 0]); s[0] += d_w; s[1] += d_c; s[2] += 1
        o = out_ph.setdefault(phase, [0.0, 0.0]); o[0] += d_w; o[1] += d_c
        prev_t = t
        if M_SOLVE.match(l): phase = "lp"
        elif M_LP.match(l): phase = "cut"
        elif M_CUT.match(l): phase = "cut"
        elif M_HEUR.match(l): phase = "post"
        elif M_RESTART.match(l): phase = "pre"
        elif M_TREE.match(l): break
        elif phase == "cut" and not M_CUT.match(l) and l.startswith("MIP: root heuristic"): phase = "heur"
    return {"phases": {k: [round(v[0], 3), round(v[1], 3)] for k, v in out_ph.items()},
            "steps": {k: [round(v[0], 3), round(v[1], 3), v[2]] for k, v in steps.items() if v[0] > 0.02},
            "tree_start": prev_t, "nlines": len(lines)}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["seed"], r["threads"]))
for p, seed, th in PLAN:
    if (p, seed, th) in done: continue
    r = one(p, seed, th)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "seed": seed, "threads": th, "extra": EXTRA, **r, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, seed, th, r["phases"], flush=True)
