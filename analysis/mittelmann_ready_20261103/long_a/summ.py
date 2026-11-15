"""long.jsonl と logs/*.stderr から表を作る。使い方: python summ.py [long.jsonl]"""
import json, re, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROG = re.compile(r"^MIP: +([0-9.]+)s nodes +(\d+) open +(\d+) lb (\S+) ub (\S+).*?(?:pool_len (\d+))?\s*(?:pool_idle \d+)?$")


def prog(path):
    out = []
    for line in open(path, errors="replace"):
        m = PROG.match(line.rstrip())
        if m:
            t, n, o, lb, ub, pl = m.groups()
            out.append((float(t), int(n), int(o), float(lb), float(ub), int(pl or 0)))
    return out


def at(pr, t):
    """時刻 t 以前の最後の経過行"""
    c = [p for p in pr if p[0] <= t]
    return c[-1] if c else None


def gap(o, b):
    if o is None or b is None or abs(o) == float("inf") or abs(b) == float("inf"):
        return None
    return abs(o - b) / max(abs(o), abs(b), 1e-10)


def main():
    f = Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "long.jsonl"
    recs = [json.loads(l) for l in open(f)]
    print("| 問題 | ソルバー | 状態 | 時間 (秒) | 返った時刻 (経過) | 目的値 | 下界 | ギャップ | ノード | 最大 RSS (MB) | ログ (KB) |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for r in recs:
        res = r["result"] or {}
        g = gap(res.get("objective"), res.get("bound"))
        print(f"| {r['problem']} | {r['solver']} | {res.get('status')} | {res.get('time') or 0:.1f} | {r['wall']:.1f} | "
              f"{res.get('objective')} | {res.get('bound')} | {'-' if g is None else f'{g:.2%}'} | {res.get('nodes')} | "
              f"{r['maxrss_mb']:.0f} | {r['log_bytes'] / 1024:.0f} |")
    print()
    print("ENOMOTO の経過 (各時刻以前の最後の経過行: ノード / 開いたノード (主のスレッドの待ち行列 + 共有の待ち行列) / RSS MB / ログ KB)")
    print()
    ts = [60, 120, 300, 600, 900, 1200, 1800]
    print("| 問題 | 上限 | " + " | ".join(f"{t} 秒" for t in ts) + " |")
    print("|---|---|" + "---|" * len(ts))
    for r in recs:
        if r["solver"] != "ours":
            continue
        lp = HERE / "logs" / f"{r['problem']}_ours_{int(r['time_limit'])}.stderr"
        pr = prog(lp)
        cells = []
        for t in ts:
            if t > r["time_limit"] + 60 or (r["result"] and (r["result"].get("time") or 0) < t - 60):
                cells.append("")
                continue
            p = at(pr, t)
            s = [x for x in r["samples"] if x[0] <= t]
            rss = f"{s[-1][1] / 1024:.0f}" if s else "-"
            lg = f"{s[-1][2] / 1024:.0f}" if s else "-"
            cells.append(f"{p[1]} / {p[2]}+{p[5]} / {rss} / {lg}" if p else f"- / - / {rss} / {lg}")
        print(f"| {r['problem']} | {int(r['time_limit'])} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
