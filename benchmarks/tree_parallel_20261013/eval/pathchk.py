import os, re, subprocess, sys
SP = "/tmp/claude-0/-home-user-enomoto-solver/89e57c1f-baeb-54a0-97e3-7cc4061b50b5/scratchpad"
V = os.environ.get("VS", "ENOMOTO_MIP_PATH_SEQ,X").split(",")
for name in sys.argv[2:]:
    for v in V:
        env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **{v: "1"})
        r = subprocess.run([f"{SP}/venv/bin/python", f"{SP}/treepar/one2.py", f"/home/user/enomoto_solver/.miplib_cache/{name}.mps", sys.argv[1], "4"], capture_output=True, text=True, env=env, cwd="/tmp")
        txt = r.stdout + r.stderr
        cr = re.findall(r"cut round (\d+): .*?sep ([\d.]+)s.*?\(([\d.]+)s\)$", txt, re.M)
        objs = re.findall(r"cut round \d+: .*?obj ([-\d.e+]+)", txt)
        sep = sum(float(a) for _, a, _ in cr)
        print(f"{name} [{v}]: {len(cr)} rounds sep {sep:.2f}s cutloop-end {cr[-1][2] if cr else '-'} first {objs[:4]} | {r.stdout.strip().splitlines()[-1] if r.stdout.strip() else r.stderr[-300:]}", flush=True)
