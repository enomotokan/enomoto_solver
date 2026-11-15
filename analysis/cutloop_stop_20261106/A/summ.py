"""res.jsonl とログから表を作る: 問題 x 設定 x 回 の状態・時間・暫定解・下界・根の下界・木に入った時刻・ノード・0 ラウンド目の扱い。
使い方: python summ.py [TL]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
tl = int(sys.argv[1]) if len(sys.argv) > 1 else None
rows = [json.loads(l) for l in open(D / "res.jsonl")]
f = lambda v: "-" if v is None else (f"{v:.6g}" if isinstance(v, float) else str(v))
print("| 問題 | 設定 | 回 | 状態 | 時間 | 暫定解 | 下界 | 根の下界 | 主のカットのループの後 (時刻) | 木に入った時刻 | ノード | カットのラウンド (LP に入ったもの) | 試作の行 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in sorted(rows, key=lambda r: (r["tl"], r["problem"], r["cfg"], r["rep"])):
    if tl and r["tl"] != tl:
        continue
    x = r["result"]
    log = D / "logs" / f"{r['problem']}_{r['cfg']}_{r['tl']}_r{r['rep']}.log"
    tree, rounds, fired, main = "-", 0, "-", "-"
    if log.exists():
        s = log.read_text(errors="replace")
        m = re.findall(r"MIP: tree search with \d+ threads \(([\d.]+)s\)", s)
        if m:
            tree = "/".join(m)
        mm = re.search(r"MIP: cut loop done: .*-> ([-\d.e+]+)", s)
        if mm:
            main = f"{float(mm.group(1)):.6g}"
        # 主のカットのループの最後のラウンドの時刻 (ラウンドの行の末尾、外したラウンドは「removed the cuts (...)」の末尾)
        i = s.find("MIP: cut loop done:")
        if i >= 0:
            tt = re.findall(r"(?:obj [-\d.e+]+|\d+\.\d+s)\) ?\(([\d.]+)s\)$|obj [-\d.e+]+ \(([\d.]+)s\)$|, ([\d.]+)s\)$", s[:i], re.M)
            if tt:
                main += f" ({[v for v in tt[-1] if v][0]}s)"
        rounds = len(re.findall(r"^MIP: cut round \d+: \d+ cuts \(of", s, re.M))
        g = re.findall(r"ENOMOTO_MIP_CUT0_RETRY: (.*)", s)
        if g:
            out = []
            for l in g:
                if "not moved" in l:
                    out.append("伸びなし→外す")
                else:
                    st = re.search(r"cap \d+(?: \(.*?\))?: (\w+) after (\d+) iters ([\d.]+)s", l)
                    sl = "全スラック" if "slack basis" in l else ""
                    out.append(f"解き直し{sl} {st.group(1)} {st.group(2)} 反復 {st.group(3)}s" if st else l[:60])
            fired = "; ".join(out)
    print(f"| {r['problem']} | {r['cfg']} | {r['rep']} | {x.get('status')} | {f(x.get('time'))} | {f(x.get('objective'))} | {f(x.get('bound'))} | {f(x.get('root_bound'))} | {main} | {tree} | {x.get('nodes')} | {rounds} | {fired} |")
