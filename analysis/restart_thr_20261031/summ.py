"""logs/ のログから、問題・設定ごとに 状態・時間・目的値・再スタートの回数・初回の判定 (新しい固定の数・根の LP の秒) をまとめる。
使い方: python summ.py [問題,...]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
want = set(sys.argv[1].split(",")) if len(sys.argv) > 1 else None
rows = defaultdict(list)
for f in sorted((D / "logs").glob("*.log")):
    p, c, rep = f.stem.rsplit("_", 2) if f.stem.count("_") >= 2 else (f.stem, "", "0")
    # 設定名に _ を含むので、末尾の回と、既知の設定名の接頭で分ける
    stem, rep = f.stem.rsplit("_", 1)
    m = re.match(r"(.+?)_((?:def|r)(?:_[a-z0-9]+)*)$", stem)
    if not m:
        continue
    p, c = m.group(1), m.group(2)
    if want and p not in want:
        continue
    t = f.read_text(errors="replace")
    js = [l for l in t.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {}
    chk = re.findall(r"restart check: newly fixed (\d+), integer columns (\d+).*?root LP ([\d.]+)s", t)
    rs = re.findall(r"MIP: restart \d+ \((\d+) of \d+ integer columns newly fixed at the root, \d+ cuts kept\) \(([\d.]+)s\)", t)
    skipped = "first restart skipped" in t
    first = f"{chk[0][0]}/{chk[0][1]} lp{chk[0][2]}" if chk else "-"
    rows[(p, c)].append((int(rep), res.get("status"), res.get("time"), res.get("objective"), "+".join(f"{a}@{float(b):.0f}" for a, b in rs) or "0", first, skipped))
for (p, c), v in sorted(rows.items()):
    v.sort()
    st = " ".join(f"{s[:3] if s else '?'}:{t:.1f}" if isinstance(t, (int, float)) else f"{s}" for _, s, t, *_ in v)
    print(f"{p:22s} {c:10s} n={len(v)} solved={sum(1 for x in v if x[1]=='optimal')} | {st} | restarts {[x[4] for x in v]} first {[x[5] for x in v]}{' skip' if any(x[6] for x in v) else ''}")
