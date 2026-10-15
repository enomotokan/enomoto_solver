import os, re, subprocess, sys
SP = "/tmp/claude-0/-home-user-enomoto-solver/89e57c1f-baeb-54a0-97e3-7cc4061b50b5/scratchpad"
tl = sys.argv[1]
print(f"{'問題':18s} {'ヒュー終':>6s} {'木開始':>6s} {'起動':>6s} {'主だけ':>6s} | {'ノード':>5s} {'強分岐':>6s} {'(LP数)':>7s} {'ノードLP':>7s} {'その他':>6s}", flush=True)
for name in sys.argv[2:]:
    env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_PROF_TREE_PAR="1")
    r = subprocess.run([f"{SP}/venv/bin/python", f"{SP}/treepar/one2.py", f"/home/user/enomoto_solver/.miplib_cache/{name}.mps", tl, "4"], capture_output=True, text=True, env=env, cwd="/tmp")
    t = r.stdout + r.stderr
    open(f"{SP}/treepar/spawn_{name}.log", "w").write(t)
    he = re.findall(r"after root heuristics: incumbent .* \(([\d.]+)s\)", t)
    m = re.search(r"TREEPAR: spawn at ([\d.]+)s tree start ([\d.]+)s nodes (\d+) sb ([\d.]+)s \((\d+) LPs, (\d+) iters\) node LP ([\d.]+)s \((\d+) LPs", t)
    if not m:
        print(f"{name:18s} no spawn", flush=True); continue
    sp, ts, nd, sb, sbl, _, nl, nll = m.groups()
    h = float(he[-1]) if he else float("nan")
    sp, ts, sb, nl = float(sp), float(ts), float(sb), float(nl)
    alone = sp - h
    print(f"{name:18s} {h:6.2f} {ts:6.2f} {sp:6.2f} {alone:6.2f} | {nd:>5s} {sb:6.2f} {sbl:>7s} {nl:7.2f} {alone - sb - nl:6.2f}", flush=True)
