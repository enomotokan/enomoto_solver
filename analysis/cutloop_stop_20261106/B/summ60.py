"""res.jsonl の 60 秒の回 (名前 s60_<設定>_r<回>) を問題・設定ごとに並べる: 状態・時間・根の下界・木に入った時刻 (求解の始めから)・前景のカットのラウンド (全段)。
根の情報はログ (logs/*.log.gz) から rootinfo.py で数え直す。使い方: python summ60.py [接頭辞 (既定 s60_)]"""
import collections, gzip, json, subprocess, sys, tempfile
from pathlib import Path
D = Path(__file__).resolve().parent
pre = sys.argv[1] if len(sys.argv) > 1 else "s60_"
rows = collections.defaultdict(lambda: collections.defaultdict(list))
for l in open(D / "res.jsonl"):
    d = json.loads(l)
    if not d["name"].startswith(pre): continue
    cfg = d["name"][len(pre):].rsplit("_r", 1)[0]
    lg = D / "logs" / f"{d['problem']}_{d['name']}.log.gz"
    with tempfile.NamedTemporaryFile("wb", suffix=".log") as f:
        f.write(gzip.open(lg).read()); f.flush()
        ro = json.loads(subprocess.run([sys.executable, str(D / "rootinfo.py"), f.name], capture_output=True, text=True).stdout)
    r = d["result"] or {}
    t = f"{r.get('time', 0):.1f}" if r.get("status") == "optimal" else f"TL [{r.get('bound', float('nan')):.6g}]"
    rows[d["problem"]][cfg].append(f"{t} / 根 {r.get('root_bound', float('nan')):.8g} / 木 {ro.get('tree_abs')} / R{ro['fg_rounds_all']}")
for p, c in rows.items():
    for cfg, xs in c.items():
        print(f"| {p} | {cfg} | " + " | ".join(xs) + " |")
