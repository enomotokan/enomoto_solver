"""ログから根の値を抜き出す: python rootsumm.py jsonl...  (logs/<stem>.<問題>.<設定>.<回>.log.gz、無ければ古い名前 <問題>.<設定>.<回>.60.log.gz)
loop1 = 最初のカットのループの最後のラウンドの LP 値 (クリークの予備の手 (clique fallback) の前)、fb = 予備の手を使ったか、
root = 根のカットの終わりの値 (cut loop done)、cuts = 最初のループの分離器ごとの 1 ラウンドあたりの候補/採用。"""
import gzip, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
def root_info(log):
    t = gzip.open(log, "rt").read()
    o = {}
    first = t.split("SEPPROF restarts 0 rounds", 1)[0]
    objs = re.findall(r"^MIP: cut round \d+: \d+ cuts .*obj ([-+0-9.e]+) \(", first, re.M)
    o["loop1"] = float(objs[-1]) if objs else None
    m = re.search(r"clique fallback: .*; (used|not used)", t)
    o["fb"] = m.group(1) == "used" if m else None
    m = re.search(r"cut loop done: .*obj ([-+0-9.e]+) -> ([-+0-9.e]+)", t)
    o["root"] = float(m.group(2)) if m else None
    m = re.search(r"SEPPROF restarts 0   cands/chosen per round:(.*)", t)
    o["cuts"] = m.group(1).strip() if m else ""
    m = re.search(r"SEPPROF restarts 0 rounds (\d+)", t)
    o["rounds"] = int(m.group(1)) if m else None
    return o
if __name__ == "__main__":
    for f in sys.argv[1:]:
        stem = Path(f).stem
        for l in open(f):
            r = json.loads(l)
            log = D / "logs" / f"{stem}.{r['problem']}.{r['cfg']}.{r['rep']}.log.gz"
            if not log.exists():
                log = D / "logs" / f"{r['problem']}.{r['cfg']}.{r['rep']}.60.log.gz"
            o = root_info(log) if log.exists() else {}
            print(f"{stem:10s} {r['problem']:20s} {r['cfg']:4s} {r['rep']} rootLP={r['info'].get('root_lp')} loop1={o.get('loop1')} fb={o.get('fb')} root={o.get('root')} rounds={o.get('rounds')} | {o.get('cuts','')}")
