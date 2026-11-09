"""GAP4: HiGHS のログ (logs/highs_<問題>.log.gz) から根の LP (最初に下界が出た行の反復と時刻)、根の終わり (処理ノードが 0 の最後の行)、
根の下界、再スタートの回数、終わりの集計 (時間・ノード・LP の反復の内訳) を抜き出す。使い方: python summ_highs.py 問題 ..."""
import gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
for p in sys.argv[1:]:
    L = gzip.open(D / "logs" / f"highs_{p}.log.gz", "rt").read().splitlines()
    rows = []
    for l in L:
        m = re.match(r"^ ?([A-Za-z])? +(\d+) +(\d+) +(\d+) +([0-9.]+)% +(\S+) +(\S+) +(\S+) +(\d+) +(\d+) +(\d+) +(\d+) +([0-9.]+)s", l)
        if m:
            rows.append(dict(proc=int(m.group(2)), bb=m.group(6), cuts=int(m.group(9)), inlp=int(m.group(10)), it=int(m.group(12)), t=float(m.group(13))))
    first_lp = next((r for r in rows if r["it"] > 0), None)
    root = [r for r in rows if r["proc"] == 0]
    last_root = root[-1] if root else None
    restarts = sum(1 for l in L if "restarting" in l)
    pres = [l for l in L if l.startswith("Presolve reductions")]
    rep = {}
    for k in ["Timing", "Nodes", "LP iterations"]:
        for i, l in enumerate(L):
            if l.strip().startswith(k):
                rep[k] = l.split()[-1] if k != "LP iterations" else l.split()[-1]
                if k == "LP iterations":
                    rep["sb"] = L[i + 1].split()[0]; rep["sep"] = L[i + 2].split()[0]; rep["heur"] = L[i + 3].split()[0]
    print(f"== {p}: root LP {first_lp and (first_lp['it'], first_lp['t'], first_lp['bb'])} | root end {last_root and (last_root['t'], last_root['bb'], last_root['cuts'], last_root['inlp'], last_root['it'])} "
          f"| restarts {restarts} | {rep}")
