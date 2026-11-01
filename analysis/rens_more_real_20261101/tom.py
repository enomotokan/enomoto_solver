"""深さ 2 の RENS (more nodes) のうち、親を良くし (acc=1) その解が親の返した解になって外側まで届いた (親も acc=1、目的値が同じ) 本数。
使い方: python tom.py ログのディレクトリ [glob]"""
import sys
from pathlib import Path
sys.argv += [] 
exec(open(Path(__file__).with_name("retries.py")).read().split("if __name__")[0])
d = Path(sys.argv[1]); pat = sys.argv[2] if len(sys.argv) > 2 else "*.log*"
for f in sorted(d.glob(pat)):
    S, order = parse(f)
    for s, p in retries(f):
        if s["depth"] != 2:
            continue
        par = S[int(s["parent"].split("/")[-1])]
        tom = bool(s.get("acc")) and bool(par.get("acc")) and par.get("obj") == s.get("obj")
        full = p is not None and p.get("nodes", -1) >= p["limit"]
        print(f"{f.name:36s} full={int(full)} retry {s.get('nodes')}/{s['limit']} {s.get('secs',0):.2f}s acc={s.get('acc')} parent={par['kind']} parent_acc={par.get('acc')} toM={int(tom)}")
