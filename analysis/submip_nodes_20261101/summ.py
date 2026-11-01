"""ログからサブ MIP の結果と根のヒューリスティクスの時間を抜き出す。
使い方: python summ.py [-v] [問題,...] [設定,...]
1 回ごとに: 状態・時間・ノード、根のヒューリスティクスの終わりの時刻 (再スタートの区間ごと)、サブ MIP の本数・時間の和・
解を出した本数・ノード上限 (以上) まで走って解のない本数・最大ノード。-v でサブ MIP を 1 本ずつ (名前 ノード 解の有無 秒)。
名前: 並列は括弧の中、逐次は直後の `root heuristic 名前:` の行、なければ `?` (木の中など)。区間は `R<再スタートの番号>`。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
args = [a for a in sys.argv[1:] if a != "-v"]
V = "-v" in sys.argv
probs = args[0].split(",") if len(args) > 0 and args[0] else None
cfgs = args[1].split(",") if len(args) > 1 else None
PAR = re.compile(r"parallel sub-MIP \(([^)]*)\): status (\w+), nodes (\d+), objective (Some\([^)]*\)|None), ([\d.]+)s")
SER = re.compile(r"^MIP:\s+sub-MIP: \d+ free columns of \d+, status (\w+), nodes (\d+), objective (Some\([^)]*\)|None), ([\d.]+)s")
RH = re.compile(r"^MIP: root heuristic ([^:]+):")
AFT = re.compile(r"after root heuristics: incumbent \S+ \(([\d.]+)s\)")

def parse(f):
    lines = f.read_text().splitlines()
    res = None
    for l in lines[::-1]:
        if l.startswith("{"):
            try: res = json.loads(l); break
            except Exception: pass
    subs, seg, afts = [], 0, []
    for i, l in enumerate(lines):
        if re.match(r"^MIP: restart \d+ \(", l):
            seg += 1
        m = AFT.search(l)
        if m: afts.append(f"R{seg}:{float(m.group(1)):.1f}")
        m = PAR.search(l)
        if m:
            subs.append((f"R{seg}", m.group(1), m.group(2), int(m.group(3)), m.group(4) != "None", float(m.group(5)), "p")); continue
        m = SER.search(l)
        if m:
            name = "?"
            for l2 in lines[i + 1:i + 6]:
                m2 = RH.match(l2)
                if m2: name = m2.group(1); break
            subs.append((f"R{seg}", name, m.group(1), int(m.group(2)), m.group(3) != "None", float(m.group(4)), "s"))
    return res, afts, subs

if __name__ == "__main__":
    fs = sorted(D.glob("*.log"))
    for f in fs:
        p, c, r = f.stem.rsplit("_", 2)
        if probs and p not in probs: continue
        if cfgs and c not in cfgs: continue
        res, afts, subs = parse(f)
        if res is None:
            print(f"{f.stem:40s} 結果なし"); continue
        st = {"optimal": "O", "time_limit": "T"}.get(res["status"], res["status"])
        nos = [s for s in subs if not s[4]]
        print(f"{f.stem:40s} {st} {res['time']:5.1f}s nodes {res['nodes']:6d} obj {res.get('objective')} | heur {' '.join(afts)} | "
              f"sub {len(subs)}本 {sum(s[5] for s in subs):5.1f}s 解 {sum(s[4] for s in subs)} 解なし {len(nos)}本 {sum(s[5] for s in nos):5.1f}s "
              f"最大ノード {max((s[3] for s in subs), default=0)}")
        if V:
            for s in subs:
                print(f"    {s[0]} {s[6]} {s[1][:28]:28s} {s[2]:10s} nodes {s[3]:4d} {'解' if s[4] else '-'} {s[5]:6.2f}s")
