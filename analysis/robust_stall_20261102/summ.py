"""robust_stall: ログから 1 回ごとの表と、問題×設定のまとめを作る。使い方: python summ.py [問題,...] > summ_all.md"""
import json, re, sys
from pathlib import Path
from collections import defaultdict
D = Path(__file__).resolve().parent
OPT = {"mc11": 11689}
rows = [json.loads(l) for l in open(D / "res.jsonl")]
want = sys.argv[1].split(",") if len(sys.argv) > 1 else None

def parse(log):
    t = log.read_text(errors="replace") if log.exists() else ""
    d = {}
    # 最初の段 (再スタート前) の裏のカット
    m = re.search(r"after root heuristics: incumbent \S+ \(([\d.]+)s\)", t)
    d["heur_end"] = float(m.group(1)) if m else None
    m = re.search(r"background cuts: determ: waited ([\d.]+)s \(safety cap [\d.]+s, (\w+), background end ([\w ]+|-)\), (\d+) rounds, (\d+) LP iters \(([\d.]+) x", t)
    if m:
        d.update(wait=float(m.group(1)), why=m.group(2), end=m.group(3), rounds=int(m.group(4)), work=float(m.group(6)))
    else:
        m = re.search(r"background cuts: waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\), (\d+) rounds", t)
        if m:
            d.update(wait=float(m.group(1)), why=m.group(3), end="", rounds=int(m.group(4)), cap=float(m.group(2)))
    m = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj \S+ -> (\S+) \(waited", t)
    if m:
        d["bg_obj"] = float(m.group(2))
    d["vetoes"] = sum(int(x) for x in re.findall(r"own stall vetoed (\d+) times", t)[:1])
    d["lpcap"] = len(re.findall(r"LP status IterationLimit, removed the cuts", t[:t.find("background cuts: determ")] if "background cuts: determ" in t else ""))
    d["p2"] = "phase 2" in (t[:t.find("background cuts: determ")] if "background cuts: determ" in t else "") and t.count("cut loop phase 2") > 0
    m = re.search(r"continued in the main cut loop: obj \S+ -> (\S+), (\d+) LP iters", t)
    if m:
        d["cont"] = float(m.group(1))
    d["restarts"] = len(re.findall(r"^MIP: restart \d+ \(", t, re.M))
    return d

table = defaultdict(list)
print("| 問題 | 設定 | 回 | 結果 | 時間 | 根の下界 | ヒューリの終わり | 裏のラウンド | 待ち | 待ちの理由 | 裏の終わり | 手間 (根の LP 倍) | 止めず続けた | 外した LP | 主の続き |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    if want and r["problem"] not in want:
        continue
    d = parse(D / "logs" / f"{r['problem']}_{r['cfg']}_{r['run']}.log")
    st = "最適" if r["status"] == "optimal" else ("時間切れ" if r["status"] == "time_limit" else r["status"])
    tm = r["time"] if r["status"] == "optimal" else 60.0
    table[(r["problem"], r["cfg"])].append((r, d))
    f = lambda v, k=".2f": "" if v is None else format(v, k)
    obj = r.get("objective")
    print(f"| {r['problem']} | {r['cfg']} | {r['run']} | {st}{'' if r['status']=='optimal' else ' ' + (f(obj, '.6g') if obj is not None else '-')} | {r['time']:.1f} | {f(r.get('root_bound'), '.6g')} | {f(d.get('heur_end'))} | {d.get('rounds', '')} | {f(d.get('wait'))} | {d.get('why', '')} | {d.get('end', '')} | {f(d.get('work'), '.1f')} | {d.get('vetoes', '')} | {d.get('lpcap', '')} | {f(d.get('cont'), '.6g')} |")
print()
print("| 問題 | 設定 | 解けた | 時間 (時間切れは 60) | SGM(+1) | 裏のカットの下界 (最初の段) | 裏のラウンド | 待ち (秒) |")
print("|---|---|---|---|---|---|---|---|")
import math
for (P, c), lst in table.items():
    ok = sum(r["status"] == "optimal" for r, _ in lst)
    ts = [min(r["time"], 60.0) if r["status"] == "optimal" else 60.0 for r, _ in lst]
    sgm = math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1
    rb = [f"{d['bg_obj']:.6g}" if 'bg_obj' in d else '-' for _, d in lst]
    rd = [str(d.get("rounds", "-")) for _, d in lst]
    wt = [f"{d['wait']:.2f}" for _, d in lst if "wait" in d]
    print(f"| {P} | {c} | {ok}/{len(lst)} | {', '.join(f'{t:.1f}' for t in ts)} | {sgm:.1f} | {', '.join(rb)} | {', '.join(rd)} | {', '.join(wt)} |")
