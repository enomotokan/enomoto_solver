"""遊び・重複探索の診断の集計: python diag_summ.py → 標準出力 (diag.txt)。logs/log_問題_t1・t4 (one.py、ENOMOTO_PROF_TREE_PAR=1) を読む。

木の段 (主が木に入ってから終わるまで、`TREEPAR: total` − `tree start`) のスレッド秒 (スレッド数 × 木の段の壁時計) を、次に分ける:
- A 分かれる前: 主が根のノード (強分枝) を処理していて従がまだいない時間 × (スレッド数 − 1) (`workers start` − `tree start`)
- B 最初のノードまで: 従が始まってから最初のノードを受け取るまで (`first`。写しの準備 `ready` + 最初の待ち。主の `TREE_HOLD_SECS` もここ)
- C 途中の待ち: 共有の待ち行列が空で待った時間 (`idle` から最初の待ちを除いたもの、主も含む)
- D 終わりの待ち: 最後に仕事が尽きて待った時間 (`tail`) と、従が先に止まった後の主の残り (`total` − 従の `end`、従が始まった後)
重複探索: 4 スレッドと 1 スレッドのノード数、最適値 (か最後の暫定解) 以上の下界のノード (`lb>=opt`、上界が最初から分かっていれば刈れたノード) の数、
最後の暫定解を見つけた時刻 (木の段の中の割合)。"""
import gzip, json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
num = r"([-0-9.e+]+)"
def parse(p, t):
    f, o = D / "logs" / f"log_{p}_t{t}.log.gz", D / "logs" / f"log_{p}_t{t}.out"
    if not o.exists(): return None
    r = json.loads(o.read_text())["result"]
    if r == "crash": return None
    d = dict(st=r["status"].lower(), time=r["time"], nodes=r.get("nodes") or 0, th=[], total=None, inc=None)
    for l in gzip.open(f, "rt"):
        w, _, l = l.partition("\t")
        m = re.search(r"TREEPAR: total " + num + r"s tree start " + num + r"s workers start " + num + "s", l)
        if m: d["total"], d["ts"], d["ws"], d["wall_total"] = float(m[1]), float(m[2]), float(m[3]), float(w)
        m = re.search(r"\| tid (\d+) ready " + num + r"s first " + num + r"s end " + num + r"s idle " + num + r"s \((\d+) waits.*lns " + num + r"s tail " + num + r"s sync " + num + r"s nodes (\d+) \(lb>=opt (\d+)\)", l)
        if m:
            d["th"].append(dict(tid=int(m[1]), ready=float(m[2]), first=float(m[3]), end=float(m[4]), idle=float(m[5]), waits=int(m[6]), lns=float(m[7]),
                                tail=float(m[8]), sync=float(m[9]), nodes=int(m[10]), waste=int(m[11])))
        if l.startswith("MIP: new incumbent"): d["inc"] = float(w)
        d["wall_end"] = float(w)
    return d
def phases(d):
    """木の段のスレッド秒と A〜D (秒)"""
    n = len(d["th"]); T = d["total"] - d["ts"]
    if n < 2 or T <= 0: return None
    pre = max(0.0, d["ws"] - d["ts"]) if d["ws"] > 0 else T
    A = (n - 1) * pre
    work = [x for x in d["th"] if x["tid"] != 0]
    # 一度もノードを受け取らなかった従 (ノード 0) は、始まってから止まるまで (end) を全部 B に入れ、その待ち (tail) は D に入れない
    B1 = sum(min(x["ready"], x["end"]) for x in work)
    B2 = sum(max(0.0, x["first"] - x["ready"]) if x["nodes"] > 0 else max(0.0, x["end"] - x["ready"]) for x in work)
    C = sum(x["idle"] for x in d["th"] if x["tid"] == 0) + sum(max(0.0, x["idle"] - max(0.0, x["first"] - x["ready"])) for x in work if x["nodes"] > 0)
    span = d["total"] - d["ws"] if d["ws"] > 0 else 0.0
    Dd = sum(x["tail"] for x in d["th"] if x["tid"] == 0 or x["nodes"] > 0) + sum(max(0.0, span - x["end"]) for x in work)
    return dict(cap=n * T, T=T, A=A, B=B1 + B2, B1=B1, B2=B2, C=C, D=Dd)
R = {p: (parse(p, 1), parse(p, 4)) for p in PROBS}
print("== 遊び (4 スレッド、種 1、60 秒。木の段に入った問題)\n")
print("| 問題 | 状態 | 全体 秒 | 木の段 秒 | A 分かれる前 | B1 写しの準備 | B2 最初の待ち | C 途中 | D 終わり | 遊びの計 | 主のノード / 全体 |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|")
tot = dict(cap=0, A=0, B=0, B1=0, B2=0, C=0, D=0); fr = []
rows = []
for p, (a, b) in R.items():
    if not b or b["total"] is None: continue
    ph = phases(b)
    if not ph: continue
    for k in tot: tot[k] += ph[k]
    idle = ph["A"] + ph["B"] + ph["C"] + ph["D"]; fr.append(idle / ph["cap"])
    n0 = sum(x["nodes"] for x in b["th"] if x["tid"] == 0); nn = sum(x["nodes"] for x in b["th"])
    rows.append((idle / ph["cap"], f"| {p} | {b['st']} | {b['time']:.1f} | {ph['T']:.1f} | " + " | ".join(f"{100 * ph[k] / ph['cap']:.0f}%" for k in ("A", "B1", "B2", "C", "D")) + f" | {100 * idle / ph['cap']:.0f}% | {n0}/{nn} |"))
for _, r in sorted(rows, reverse=True): print(r)
c = tot["cap"]
print(f"\n全問の合計 (スレッド秒の重み): A {100 * tot['A'] / c:.1f}% B {100 * tot['B'] / c:.1f}% (B1 {100 * tot['B1'] / c:.1f}% B2 {100 * tot['B2'] / c:.1f}%) C {100 * tot['C'] / c:.1f}% D {100 * tot['D'] / c:.1f}% 計 {100 * (tot['A'] + tot['B'] + tot['C'] + tot['D']) / c:.1f}% "
      f"(問題 {len(fr)}、問題ごとの割合の平均 {100 * sum(fr) / max(1, len(fr)):.1f}%、木の段のスレッド秒の計 {c:.0f})")
print("\n== 重複探索 (1 スレッドと 4 スレッドのノード数、種 1。両方とも木に入った問題を 4T / 1T の大きい順)\n")
print("| 問題 | 1T 状態 秒 ノード | 4T 状態 秒 ノード | ノード 4T/1T | lb>=opt 1T | lb>=opt 4T | 最後の暫定解の時刻 (全体の割合) 1T | 4T |")
print("|---|---|---|---:|---:|---:|---:|---:|")
L = []
for p, (a, b) in R.items():
    if not a or not b or a["nodes"] < 1 or b["nodes"] < 1: continue
    w1 = sum(x["waste"] for x in a["th"]); w4 = sum(x["waste"] for x in b["th"])
    L.append((b["nodes"] / a["nodes"], p, a, b, w1, w4))
lr = [math.log(x[0]) for x in L if x[2]["st"] == "optimal" and x[3]["st"] == "optimal"]
for r, p, a, b, w1, w4 in sorted(L, reverse=True):
    inc = lambda d: f"{d['inc'] / d['wall_end']:.2f}" if d["inc"] is not None else "—"
    print(f"| {p} | {a['st']} {a['time']:.1f} {a['nodes']} | {b['st']} {b['time']:.1f} {b['nodes']} | {r:.2f} | {w1} | {w4} | {inc(a)} | {inc(b)} |")
print(f"\n両方とも最適で終えた {len(lr)} 問のノード 4T/1T の幾何平均 {math.exp(sum(lr) / max(1, len(lr))):.3f}")
