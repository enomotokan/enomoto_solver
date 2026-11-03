"""mc11_wait1: ログから 1 回ごとの表と、問題×設定のまとめを作る。使い方: python summ.py [問題,...] [設定,...] > summ_all.md"""
import json, math, re, sys
from pathlib import Path
from collections import defaultdict
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "res.jsonl")]
want = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
wantc = sys.argv[2].split(",") if len(sys.argv) > 2 else None


def parse(log):
    t = log.read_text(errors="replace") if log.exists() else ""
    d = {}
    m = re.search(r"after root heuristics: incumbent \S+ \(([\d.]+)s\)", t)
    d["heur_end"] = float(m.group(1)) if m else None
    m = re.search(r"background cuts: determ: waited ([\d.]+)s \(safety cap [\d.]+s, (\w+), background end ([\w ]+|-)\), (\d+) rounds, (\d+) LP iters", t)
    if m:
        d.update(wait=float(m.group(1)), why=m.group(2), end=m.group(3), rounds=int(m.group(4)), iters=int(m.group(5)))
    else:
        m = re.search(r"background cuts: waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\), (\d+) rounds", t)
        if m:
            d.update(wait=float(m.group(1)), why=m.group(3), end="", rounds=int(m.group(4)))
    m = re.search(r"background cuts: (\d+) rounds, (\d+) cut rows \(was \d+\), obj \S+ -> (\S+) \(waited", t)
    if m:
        d["bg_obj"] = float(m.group(3)); d["bg_rows"] = int(m.group(2))
    m = re.search(r"background cuts: rounds so far \[([^\]]*)\]", t)
    if m:
        v = re.findall(r"([\d.e+-]+) at", m.group(1))
        d["r1"] = float(v[0]) if v else None
        d["r10"] = float(v[9]) if len(v) >= 10 else None
        d["r30"] = float(v[29]) if len(v) >= 30 else None
    pre = t[:t.find("background cuts: determ: waited")] if "background cuts: determ: waited" in t else t
    d["lpcap"] = len(re.findall(r"removed the cuts", pre))
    m = re.search(r"continued in the main cut loop: obj \S+ -> (\S+)", t)
    if m:
        d["cont"] = float(m.group(1))
    d["restarts"] = len(re.findall(r"^MIP: restart \d+ \(", t, re.M))
    m = re.findall(r"tree search threads: worker nodes (\d+)", t)
    d["wnodes"] = int(m[-1]) if m else None
    return d


table = defaultdict(list)
print("| 問題 | 設定 | 回 | 結果 | 時間 | ノード | 根の下界 | ヒューリの終わり | 裏のラウンド | 裏の下界 | 1 ラウンド目 | 10 | 30 | 待ち | 待ちの理由 | 裏の終わり | 外した LP | 主の続き | 再スタート |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    if want and r["problem"] not in want:
        continue
    if wantc and r["cfg"] not in wantc:
        continue
    d = parse(D / "logs" / f"{r['problem']}_{r['cfg']}_{r['run']}.log")
    st = "最適" if r["status"] == "optimal" else ("時間切れ" if r["status"] == "time_limit" else r["status"])
    table[(r["problem"], r["cfg"])].append((r, d))
    f = lambda v, k=".2f": "" if v is None else format(v, k)
    obj = r.get("objective")
    print(f"| {r['problem']} | {r['cfg']} | {r['run']} | {st}{'' if r['status']=='optimal' else ' ' + (f(obj, '.6g') if obj is not None else '-')} | {r['time']:.1f} | {r.get('nodes','')} | {f(r.get('root_bound'), '.6g')} | {f(d.get('heur_end'))} | {d.get('rounds', '')} | {f(d.get('bg_obj'), '.6g')} | {f(d.get('r1'), '.6g')} | {f(d.get('r10'), '.6g')} | {f(d.get('r30'), '.6g')} | {f(d.get('wait'))} | {d.get('why', '')} | {d.get('end', '')} | {d.get('lpcap', '')} | {f(d.get('cont'), '.6g')} | {d.get('restarts')} |")
print()
print("| 問題 | 設定 | 解けた | 時間 (時間切れは 60) | SGM(+1) | ノード | 根の下界 | 裏のラウンド | 待ち (秒) |")
print("|---|---|---|---|---|---|---|---|---|")
for (P, c), lst in table.items():
    ok = sum(r["status"] == "optimal" for r, _ in lst)
    ts = [min(r["time"], 60.0) if r["status"] == "optimal" else 60.0 for r, _ in lst]
    sgm = math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1
    rb = [f"{r['root_bound']:.6g}" if r.get('root_bound') is not None else '-' for r, _ in lst]
    nd = [str(r.get("nodes", "-")) for r, _ in lst]
    rd = [str(d.get("rounds", "-")) for _, d in lst]
    wt = [f"{d['wait']:.2f}" for _, d in lst if "wait" in d]
    print(f"| {P} | {c} | {ok}/{len(lst)} | {', '.join(f'{t:.1f}' for t in ts)} | {sgm:.1f} | {', '.join(nd)} | {', '.join(rb)} | {', '.join(rd)} | {', '.join(wt)} |")
