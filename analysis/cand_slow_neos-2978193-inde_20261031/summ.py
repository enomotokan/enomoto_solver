"""run.py のログを (早め: E は逐次のヒューリスティクスの後の早めの再スタート、J は試作 ENOMOTO_MIP_EARLY_RESTART_JOIN のもの)設定ごとにまとめる: 全体の秒 (時間切れは 60)、最初の根の長さ (再スタートの時刻)、早めの再スタートの有無、
最初の根の RENS・RINS のサブ MIP の秒、木の秒。使い方: python summ.py [問題] [設定,...]"""
import re, sys, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
prob = sys.argv[1] if len(sys.argv) > 1 else "neos-2978193-inde"
want = sys.argv[2].split(",") if len(sys.argv) > 2 else None
rows = {}
for f in sorted(D.glob(f"{prob}_*_*.log")):
    m = re.match(rf"{re.escape(prob)}_(.+)_(\d+)\.log$", f.name)
    cfg, rep = m.group(1), int(m.group(2))
    if want and cfg not in want:
        continue
    s = f.read_text()
    tot = re.findall(r'"time": ([\d.]+)', s); stt = re.findall(r'"status": "(\w+)"', s)
    t = float(tot[-1]) if tot else 60.0
    if not stt or stt[-1] != "optimal" or t > 60:
        t = 60.0
    first = s.split("MIP: restart 1 (")[0]
    rst = re.search(r"^MIP: restart 1 \(.*\) \(([\d.]+)s\)", s, re.M)
    er = re.search(r"^MIP: early restart: .* after (.+?) \(", first, re.M)
    subs = [float(x) for x in re.findall(r"^MIP:   (?:parallel )?sub-MIP.*?, ([\d.]+)s$", first, re.M)]
    ts = re.findall(r"^MIP: tree search with \d+ threads \(([\d.]+)s\)", s, re.M); fe = re.findall(r"^MIP: finish start \w+ \(([\d.]+)s\)", s, re.M)
    tree = float(fe[-1]) - float(ts[-1]) if ts and fe else float("nan")
    rows.setdefault(cfg, []).append((rep, t, float(rst.group(1)) if rst else float("nan"), ("J" if "parallel sub-MIPs" in er.group(1) else "E") if er else "-", sum(s_ for s_ in subs if s_ >= 0.2), tree))
order = list(want) if want else sorted(rows, key=lambda c: (c != "def", c != "cand", c))
print(f"{'設定':12s} {'回':>2s} {'平均':>6s} {'中央':>6s} {'最初の根 平均':>8s} {'早め':>4s} {'根1 sub 秒':>8s} {'木 平均':>6s}  回ごと (秒/最初の根/早め)")
for c in order:
    if c not in rows:
        continue
    r = sorted(rows[c])
    ts_ = [x[1] for x in r]
    print(f"{c:12s} {len(r):2d} {st.mean(ts_):6.1f} {st.median(ts_):6.1f} {st.mean(x[2] for x in r):8.1f} {sum(x[3]=='E' for x in r):2d}/{len(r):<2d}"
          f" {st.mean(x[4] for x in r):8.1f} {st.mean(x[5] for x in r):6.1f}  " + " ".join(f"{x[1]:.1f}/{x[2]:.1f}{x[3]}" for x in r))
