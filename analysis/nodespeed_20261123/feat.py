"""適応形 (ENOMOTO_MIP_NODEROWS_ADAPT) の特徴の候補を測る (README 8 節): python feat.py <出力 jsonl> <秒> <設定名> <環境変数=値,...|-> <問題 ...>
1 スレッド・種 1 で ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_TREE_LOG=1 をつけて解き、ログから数える。リポジトリの根で回す。
- 閉じた葉の理由 (ENODE の r=): 実行不能 (prop・lpinf・sbprop・sbinherit) と下界 (pop_lb・lpobj・redcost・tubr・int)、ほか (sb)
- ノードの LP の平均の行 (node diag) と木の始まりの LP の行 (root LP size の rows + cut loop done の in LP)
- pse の効き目 (POOL_SEL e の行: improved・not improved)、目的の係数が全部 0 か (MPS を読んで数える)"""
import json, os, re, subprocess, sys
from collections import Counter

out, tl, name, envs, probs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
INF = {"prop", "lpinf", "sbprop", "sbinherit"}
BND = {"pop_lb", "lpobj", "redcost", "tubr", "int"}
for p in probs:
    env = dict(os.environ, ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE_LOG="1")
    if envs != "-":
        for kv in envs.split(","):
            k, v = kv.split("=", 1)
            env[k] = v
    r = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps", "--time-limit", tl, "--threads", "1"],
                       env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    c = Counter(m.group(1) for m in re.finditer(r"^ENODE d=\d+ r=(\w+)", r.stderr, re.M))
    inf, bnd = sum(c[k] for k in INF), sum(c[k] for k in BND)
    m_rows = re.search(r"root LP size: rows (\d+)", r.stderr)
    in_lp = re.search(r"cut loop done: \d+ cuts added, (\d+) in LP", r.stderr)
    nd = re.search(r"node diag: nodes \d+ node LPs (\d+) mean rows ([\d.]+) iters \d+ \(([\d.]+)/LP\)", r.stderr)
    pse = re.search(r"e: improved (\d+) not improved (\d+)", r.stderr)
    start_rows = (int(m_rows.group(1)) + int(in_lp.group(1))) if m_rows and in_lp else None
    d = dict(problem=p, cfg=name, status=res["status"], time=round(res["time"], 2), nodes=res["nodes"], leaves=dict(c),
             inf_frac=round(inf / max(1, inf + bnd), 4), n_inf=inf, n_bnd=bnd,
             start_rows=start_rows, mean_rows=float(nd.group(2)) if nd else None, iters_per_lp=float(nd.group(3)) if nd else None,
             row_growth=round(float(nd.group(2)) / start_rows, 3) if nd and start_rows else None,
             pse_improved=int(pse.group(1)) if pse else None, pse_not=int(pse.group(2)) if pse else None)
    print(json.dumps(d), flush=True)
    with open(out, "a") as f:
        f.write(json.dumps(d) + "\n")
