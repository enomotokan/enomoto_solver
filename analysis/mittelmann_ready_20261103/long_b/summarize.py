"""long_ours.jsonl / long_highs.jsonl と ENOMOTO のログ (LOGDIR) から表を作る (2026-11-03)。
使い方: python analysis/mittelmann_ready_20261103/long_b/summarize.py [LOGDIR]
- ENOMOTO のログの進み行 `MIP: <秒>s nodes <N> open <M> lb ... ub ...` (主スレッド、2 秒ごと) から、ノード数・開いたノード数の推移を拾う。
  ログの行の時刻は分枝限定法の始め (再スタート後はその後) からなので、推移は行の順で、測った経過時間 (series) の割合で引き当てる。
- 各ログの要点 (進み行の間引き・末尾 40 行) は long_b/logs/ に書く。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
LOGDIR = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/claude-0/longb_logs")
(D / "logs").mkdir(exist_ok=True)
PROG = re.compile(r"^MIP:\s+([0-9.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")

def load(kind):
    f = D / f"long_{kind}.jsonl"
    return {json.loads(l)["problem"]: json.loads(l) for l in f.read_text().splitlines()} if f.exists() else {}

def gap(o, b):
    if o is None or b is None: return None
    return abs(o - b) / max(abs(o), 1.0)

ours, highs = load("ours"), load("highs")
rows, trend = [], []
for p, r in ours.items():
    res = r["result"]; tl = r["time_limit"]
    log = LOGDIR / f"ours_{p}_{int(tl)}.log"
    prog = []
    if log.exists():
        lines = log.read_text(errors="replace").splitlines()
        prog = [m.groups() for m in map(PROG.match, lines) if m]
        keep = prog[:: max(1, len(prog) // 40)]
        (D / "logs" / f"ours_{p}_{int(tl)}_summary.txt").write_text(
            f"# {log.name}: {len(lines)} 行・{r['log_bytes']} バイト、進み行 {len(prog)} 本 (間引いて {len(keep)} 本)\n"
            + "\n".join("MIP: %ss nodes %s open %s lb %s ub %s" % g for g in keep) + "\n\n# 末尾 40 行\n"
            + "\n".join(l[:300] for l in lines[-40:]) + "\n")
    s = [x for x in r["series"] if x[1] > 0]  # 終わった後の 0 を除く
    # 1/4・1/2・3/4・終わりの RSS とログの大きさ
    q = lambda f: s[min(len(s) - 1, int(len(s) * f))] if s else [None, None, None]
    trend.append((p, [q(0.25), q(0.5), q(0.75), s[-1] if s else None]))
    mx_nodes = max((int(g[1]) for g in prog), default=0); mx_open = max((int(g[2]) for g in prog), default=0)
    last_open = int(prog[-1][2]) if prog else None
    h = highs.get(p, {}).get("result", {})
    rows.append((p, res, r, mx_open, last_open, h, highs.get(p)))

print("| 問題 | ENOMOTO 状態 | 時間 [s] | 目的値 | 下界 | ギャップ | ノード | 開いたノード (最大/最後) | 最大 RSS [MB] | ログ [KB] | 警告 | HiGHS 状態 | 時間 [s] | 目的値 | 下界 | ギャップ | ノード | 最大 RSS [MB] |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|")
f = lambda v, k=".9g": "-" if v is None else format(v, k)
for p, res, r, mo, lo, h, hr in rows:
    g = gap(res.get("objective"), res.get("bound")); hg = gap(h.get("objective"), h.get("bound"))
    print(f"| {p} | {res.get('status')} | {f(res.get('time'), '.1f')} | {f(res.get('objective'))} | {f(res.get('bound'))} | {f(g, '.2e')} | {res.get('nodes')} | {mo}/{lo} "
          f"| {r['maxrss_mb']:.0f} | {r['log_bytes']/1024:.0f} | {r['warnings']} | {h.get('status', '-')} | {f(h.get('time'), '.1f')} | {f(h.get('objective'))} | {f(h.get('bound'))} "
          f"| {f(hg, '.2e')} | {h.get('nodes', '-')} | {f(hr['maxrss_mb'] if hr else None, '.0f')} |")
print("\nRSS [MB]・ログ [KB] の推移 (経過 1/4・1/2・3/4・終わり、[経過秒, RSS, ログ])")
for p, t in trend:
    print(f"- {p}: " + ", ".join("-" if x is None else f"[{x[0]:.0f}s, {x[1]:.0f}, {x[2]/1024:.0f}]" for x in t))
