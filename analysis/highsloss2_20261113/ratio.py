"""HIGHSLOSS2 D2: 1 問を cand1114 (+ 足す環境変数) の 1 スレッドで解き、ノードの最初の LP の反復を
潜りの子 / 待ち行列から取ったノードで比べる (ENOMOTO_PROF_TREE_PAR の TREEPAR 行と ENOMOTO_MIP_RESTORE_DIAG の RESTDIAG 行)。
python ratio.py 名札 秒 問題... [-- 名前=値 ...]   結果は ratio.jsonl に 1 問 1 行 (ログは logs/eno_<問題>_<名札>.log.gz)"""
import gzip, json, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
tag, tl = sys.argv[1], sys.argv[2]
rest = sys.argv[3:]
probs, extra = (rest[:rest.index("--")], rest[rest.index("--") + 1:]) if "--" in rest else (rest, [])
for p in probs:
    subprocess.run([sys.executable, str(D / "eno.py"), p, tag, tl, "1", "ENOMOTO_PROF_TREE_PAR=1", "ENOMOTO_MIP_RESTORE_DIAG=1", *extra], check=False, capture_output=True)
    txt = gzip.open(D / "logs" / f"eno_{p}_{tag}.log.gz", "rt").read()
    m = re.search(r"first LP \[dive (\d+) ([\d.]+)it ([\d.]+)ms \| queue (\d+) ([\d.]+)it ([\d.]+)ms", txt)
    rs = re.search(r"basis restores (\d+) \(basic count mismatch (\d+)\)", txt)
    cls = {}
    for k, a, d, mm, it in re.findall(r"RESTDIAG: tid \d+ kind (\d) added (\d+) deleted (\d+) mismatch (\w+) excess \d+ rows \d+ iters (\d+)", txt):
        if k != "1":
            continue
        c = "same" if a == "0" and d == "0" else ("mismatch" if mm == "true" else "rows")
        cls.setdefault(c, [0, 0])
        cls[c][0] += 1
        cls[c][1] += int(it)
    nd = re.search(r"node diag: nodes (\d+) node LPs (\d+) mean rows ([\d.]+)", txt)
    row = {"problem": p, "tag": tag, "tl": tl, "extra": extra}
    if m:
        dv, di, dm, qn, qi, qm = m.groups()
        row.update(dive=int(dv), dive_it=float(di), dive_ms=float(dm), queue=int(qn), queue_it=float(qi), queue_ms=float(qm),
                   ratio=round(float(qi) / max(float(di), 1e-9), 1))
    if rs:
        row.update(restores=int(rs[1]), mismatch=int(rs[2]))
    row["queue_restored"] = {c: [v[0], round(v[1] / v[0], 1)] for c, v in cls.items()}
    if nd:
        row.update(nodes=int(nd[1]), mean_rows=float(nd[3]))
    with open(D / "ratio.jsonl", "a") as f:
        f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
