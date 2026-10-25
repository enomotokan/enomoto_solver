"""sizes.jsonl から比較表 (Markdown) を作る。使い方: python3 table.py sizes.jsonl
e = ENOMOTO のクリーク行の拡張の後 (根の LP を解く問題)。差は ENOMOTO − HiGHS。LP 閉じ = (前処理後の LP − 元の LP) の差 (最小化)"""
import sys, json
R = [json.loads(l) for l in open(sys.argv[1]) if l.startswith("{")]
def f(v): return "—" if v is None else f"{v:.6g}"
print("| 問題 | 元 (行×列) | HiGHS (行×列, 非零) | ENOMOTO (行×列, 非零) | 行の差 | 列の差 | 非零の差 | 0-1/整数/連続 (H) | (E) | LP 元 | LP H | LP E | 時間 H/E |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
tot = dict(hr=0, hc=0, hn=0, er=0, ec=0, en=0)
for r in sorted(R, key=lambda r: r["name"]):
    o, h, e = r["orig"], r["h"], r["e2"]
    if h is None or e is None:
        print(f"| {r['name']} | {o['rows']}×{o['cols']} | {r['hstatus']} | {'—' if e is None else str(e['rows'])+'×'+str(e['cols'])} |||||||||"); continue
    for k, a, b in (("r", "rows", "rows"), ("c", "cols", "cols"), ("n", "nnz", "nnz")):
        tot["h" + k] += h[a]; tot["e" + k] += e[b]
    dr, dc, dn = e["rows"] - h["rows"], e["cols"] - h["cols"], e["nnz"] - h["nnz"]
    print(f"| {r['name']} | {o['rows']}×{o['cols']} | {h['rows']}×{h['cols']}, {h['nnz']} | {e['rows']}×{e['cols']}, {e['nnz']} | {dr:+d} | {dc:+d} | {dn:+d} | "
          f"{h['bin']}/{h['int']}/{h['cont']} | {e['bin']}/{e['int']}/{e['cont']} | {f(o['lp'])} | {f(h['lp'])} | {f(e['lp'])} | {r['ht']}/{r['et']} |")
print(f"\n合計: HiGHS {tot['hr']} 行 × {tot['hc']} 列, 非零 {tot['hn']} / ENOMOTO {tot['er']} 行 × {tot['ec']} 列, 非零 {tot['en']}")
