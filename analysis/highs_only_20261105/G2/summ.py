"""res.jsonl (600 秒) と res58.jsonl (58 問・60 秒) の表を出す。使い方: python summ.py"""
import gzip, json, math, re
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
f = lambda x: "-" if x is None else f"{x:.10g}"
print("| 問題 | 設定 | 状態 | 時間 | 下界 | 暫定解 | ノード | 根の再スタート | 木の中の再スタート (固定数) | 見送り |")
print("|---|---|---|---|---|---|---|---|---|---|")
for l in open(D / "res.jsonl"):
    e = json.loads(l); r = e["result"] or {}
    err = gzip.open(D / "logs" / f"{e['name']}_{e['problem']}.err.gz", "rt", errors="replace").read()
    tree = re.findall(r"MIP: TREE restart: (\d+) newly", err)
    nall = len(re.findall(r"^MIP: restart \d+ \(", err, re.M))
    st = "最適" if r.get("status") == "optimal" else "時間切れ" if r.get("status") == "time_limit" else r.get("status")
    print(f"| {e['problem']} | {e['name']} | {st} | {r.get('time', 0):.1f} | {f(r.get('bound'))} | {f(r.get('objective'))} | {r.get('nodes')} | {nall - len(tree)} | {len(tree)} ({', '.join(tree)}) | {err.count('skipped: newly fixed')} |")
p58 = D / "res58.jsonl"
if p58.exists():
    res = defaultdict(dict)
    for l in open(p58):
        e = json.loads(l); res[e["problem"]][e["cfg"]] = e["result"]
    cfgs = ["def", "ho2"]
    keys = [k for k in res if all(c in res[k] for c in cfgs)]
    ok = lambda r: r.get("status") == "optimal"
    print(f"\n58 問 ({len(keys)} 問そろった分)\n\n| 設定 | 解けた数 | SGM (ずらし 10 秒) | WARNING |\n|---|---|---|---|")
    for c in cfgs:
        rs = [res[k][c] for k in keys]
        t = [min(r["time"], 60.0) if ok(r) else 60.0 for r in rs]
        sgm = math.exp(sum(math.log(x + 10) for x in t) / len(t)) - 10
        print(f"| {c} | {sum(ok(r) for r in rs)} / {len(rs)} | {sgm:.2f} | {sum(r.get('mip_warnings', 0) for r in rs)} |")
    print("\n刻みが変わった問題 (ho2_step):", [k for k in keys if res[k]["ho2"].get("ho2_step")])
    print("HO2_GOBJ が締めた問題 (回数):", [(k, res[k]["ho2"].get("ho2_gobj")) for k in keys if res[k]["ho2"].get("ho2_gobj")])
    print("\n片方だけ解けた・25% 以上違う・暫定解が違う:")
    for k in keys:
        a, b = res[k]["def"], res[k]["ho2"]
        if ok(a) != ok(b) or (ok(a) and ok(b) and abs(a["time"] - b["time"]) > 0.25 * max(a["time"], b["time"]) and abs(a["time"] - b["time"]) > 2) or (not ok(a) and not ok(b) and a.get("objective") != b.get("objective")):
            print(f"  {k}: def {a.get('status')} {f(a.get('time'))} 下界 {f(a.get('bound'))} 暫定解 {f(a.get('objective'))} / ho2 {b.get('status')} {f(b.get('time'))} 下界 {f(b.get('bound'))} 暫定解 {f(b.get('objective'))}")
        if ok(a) and ok(b) and abs(a["objective"] - b["objective"]) > 1e-6 * (1 + abs(a["objective"])):
            print(f"  食い違い {k}: {a['objective']} / {b['objective']}")
