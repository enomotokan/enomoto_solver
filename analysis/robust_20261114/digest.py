"""ログの道しるべを 1 回 1 行に: python digest.py 問題 [スレッド=1]  (logs/<問題>_t<スレッド>_p*s*.log.gz を全部)
出すもの: 前処理後の大きさ、根の LP の勝ち手 (4 スレッド)、根の LP の値、根のカットのラウンド数と終わりの値、再スタートの数、
木に入る時点の下界・暫定解 (TRSTAT)、暫定解の出どころの並び (時刻 [出どころ] 値)、終わりのノード数・状態・秒。
時刻は各求解の始めからの秒 (再スタートで 0 に戻る) なので、出どころの並びには再スタートの番号 R を付ける。"""
import gzip, re, sys, json
from pathlib import Path
D = Path(__file__).resolve().parent
def digest(path):
    d = dict(rounds=[], inc=[], restarts=0, winner=[], rootlp=[], cutend=[], tree=None, nodes=None, ret=None, pres=None)
    r = 0
    for l in gzip.open(path, "rt", errors="replace"):
        if l.startswith("MIP: presolve: rows") and d["pres"] is None:
            m = re.search(r"rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", l); d["pres"] = m and f"{m[2]}x{m[4]}"
        elif "finished first" in l:
            d["winner"].append("IPM" if "interior" in l else "SPX")
        elif l.startswith("MIP: root LP ") and "iters" in l and "size" not in l and "basis" not in l:
            m = re.search(r"root LP ([-\d.e+]+)", l); m and d["rootlp"].append(float(m[1]))
        elif re.match(r"MIP: cut round \d+: \d+ cuts", l):
            m = re.search(r"obj ([-\d.e+]+)", l); m and d["rounds"].append((r, float(m[1])))
        elif l.startswith("MIP: cut loop done"):
            m = re.search(r"-> ([-\d.e+]+)", l); m and d["cutend"].append(float(m[1]))
        elif l.startswith("MIP: new incumbent"):
            m = re.search(r"incumbent ([-\d.e+]+) at node (\d+) \(([\d.]+)s\) \[(.*)\]", l)
            if m: d["inc"].append((r, float(m[3]), m[4], float(m[1]), int(m[2])))
        elif re.match(r"MIP: restart \d+ \(", l) or l.startswith("MIP: TREE restart") or "tree restart" in l.lower() and "restart" in l[:40]:
            d["restarts"] += 1; r += 1
        elif l.startswith("TRSTAT: tree start"):
            m = re.search(r"lb ([-\d.e+inf]+) ub ([-\d.e+inf]+)", l); m and (d.__setitem__("tree", (r, m[1], m[2])))
        elif re.match(r"MIP:\s+[\d.]+s nodes", l):
            m = re.search(r"nodes\s+(\d+)", l); d["nodes"] = int(m[1])
        elif l.startswith("MIP: solve returned"):
            d["ret"] = l.strip()[len("MIP: solve returned "):]
    return d
def fmt(name, d):
    nr = {}
    for rr, _ in d["rounds"]: nr[rr] = nr.get(rr, 0) + 1
    srcs = []
    for rr, t, s, v, nd in d["inc"]:
        srcs.append(f"R{rr}:{t:.1f}s[{s}]{v:.6g}@{nd}")
    return (f"{name}: pres {d['pres']} win {''.join(x[0] for x in d['winner'])} rootLP {['%.6g' % x for x in d['rootlp'][:3]]} "
            f"rounds {nr} cutend {['%.6g' % x for x in d['cutend'][:3]]} restarts {d['restarts']} tree {d['tree']} nodes {d['nodes']} "
            f"ret {d['ret']}\n    inc {' '.join(srcs[:4])}{' ... ' + ' '.join(srcs[-3:]) if len(srcs) > 7 else ' ' + ' '.join(srcs[4:])}")
if __name__ == "__main__":
    p = sys.argv[1]; th = sys.argv[2] if len(sys.argv) > 2 else "1"
    for f in sorted((D / "logs").glob(f"{p}_t{th}_p*s*.log.gz")):
        print(fmt(f.name.split(f"_t{th}_")[1][:-7], digest(f)))
