"""root.py のログの集計: python rootsum.py [札の接頭辞]。最初の根のカットのループ (再スタートの前) について、
根の下界・残りギャップの割合 ((最適値 − 下界) / (最適値 − 最初の LP))、ループのラウンド数・LP に残ったカットの本数・その非零 (最後のラウンドの
LP の行 − 根の LP の行) × 平均の非零・候補の合計・分離の秒の合計・LP の反復の合計・ループの手間 (最後の root cut time cap check の clock、
手間の秒)・ループの経過秒、と全体の結果 (状態・ノード・秒) を出す。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
from root import PROBS  # noqa: E402
OPT = {"p200x1188c": 15078, "mc11": 11689, "beasleyC3": 754, "10teams": 924, "exp-1-500-5-5": 65887, "nw04": 16862, "qiu": -132.873136947, "timtab1": 764772}
def parse(path):
    L = path.read_text().splitlines()
    d = {"rounds": 0, "cands": 0, "sep": 0.0, "iters": 0, "clock": None}
    rows0 = None
    for l in L:
        m = re.search(r"root LP size: rows (\d+)", l)
        if m and rows0 is None:
            rows0 = int(m.group(1))
        m = re.search(r"MIP: root LP (-?[\d.e+-]+) \(", l)
        if m and "lp0" not in d:
            d["lp0"] = float(m.group(1))
        m = re.search(r"cut round (\d+): (\d+) cuts \(of (\d+), sep ([\d.]+)s, LP (\d+) iters\), LP rows (\d+) \(cut rows mean nnz (\d+)\), obj (\S+) \(([\d.]+)s\)", l)
        if m:
            d["rounds"] += 1; d["cands"] += int(m.group(3)); d["sep"] += float(m.group(4)); d["iters"] += int(m.group(5))
            d["lprows"] = int(m.group(6)); d["mnnz"] = int(m.group(7)); d["loopsec"] = float(m.group(9))
        m = re.search(r"root cut time cap check: clock ([\d.]+)s", l)
        if m:
            d["clock"] = float(m.group(1))
        m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP, .*obj (\S+) -> (\S+)", l)
        if m:
            d["added"] = int(m.group(1)); d["inlp"] = int(m.group(2)); d["first"] = float(m.group(3)); d["bound"] = float(m.group(4))
            break
    d["ncut"] = (d.get("lprows", rows0 or 0) - (rows0 or 0))
    d["nnz"] = d["ncut"] * d.get("mnnz", 0)
    r = L[-1]
    d["res"] = json.loads(r.split(" ", 1)[1]) if r.startswith("@@RESULT") else None
    return d
pre = sys.argv[1] if len(sys.argv) > 1 else ""
print("| 問題 | 設定 | 根の下界 | 残り | ラウンド | LP のカット | 非零 | 候補 | 分離秒 | LP 反復 | 手間秒 | ループ秒 | 全体 |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
for p in PROBS:
    for c in CFGS:
        f = D / "logs" / f"{pre}{c}_{p}.log"
        if not f.exists():
            continue
        d = parse(f)
        if "bound" not in d:
            print(f"| {p} | {c} | (根のループなし) |"); continue
        lp0 = d.get("lp0", d["first"]); opt = OPT[p]
        gap = (opt - d["bound"]) / (opt - lp0) * 100 if opt != lp0 else 0.0
        r = d["res"] or {}
        tot = f"{r.get('status')}・{r.get('nodes')}・{r.get('time', 0):.1f}" if r else "-"
        clk = f"{d['clock']:.2f}" if d["clock"] is not None else "-"
        print(f"| {p} | {c} | {d['bound']:.6g} | {gap:.2f}% | {d['rounds']} | {d['ncut']} | {d['nnz']} | {d['cands']} | {d['sep']:.2f} | {d['iters']} | {clk} | {d.get('loopsec', 0):.2f} | {tot} |")
