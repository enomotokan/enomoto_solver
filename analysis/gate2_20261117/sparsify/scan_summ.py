"""scan.jsonl のまとめ: python scan_summ.py > scan_summ.txt。入る問題 (gate ≥ 0.5) と、書き換えのある問題の f、def と spls の前処理の後の大きさの一致。"""
import json, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for l in open(D / "scan.jsonl"):
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r
both = [p for p in R if "def" in R[p] and "spls" in R[p]]
print(f"問題 {len(R)}、2 回そろった {len(both)}")
gin, ch, mism, bad = [], [], [], []
for p in sorted(both):
    d, s = R[p]["def"], R[p]["spls"]
    if d.get("err") or s.get("err") or d.get("timeout") or s.get("timeout") or not d.get("pre") or not s.get("pre"):
        bad.append((p, d.get("err") or ("timeout" if d.get("timeout") else ""), s.get("err") or ("timeout" if s.get("timeout") else ""), d.get("pre"), s.get("pre"))); continue
    st = s["stats"][-1] if s["stats"] else None
    same = d["pre"][1:] == s["pre"][1:]
    if st and int(st[3]) > 0:
        f = int(st[5]) / int(st[3]); g = float(st[8])
        ch.append((p, s["mitt"], int(st[3]), int(st[4]), int(st[5]), f, g, d["pre"][1:], s["pre"][1:]))
        if g >= 0.5: gin.append(p)
        elif not same: mism.append((p, d["pre"][1:], s["pre"][1:]))
    elif not same:
        mism.append((p, d["pre"][1:], s["pre"][1:]))
print(f"\n疎化が行を書き換えた問題 {len(ch)} (f の大きい順。M = Mittelmann 186 問に入る):")
for p, m, c, b, e, f, g, dp, sp in sorted(ch, key=lambda x: -x[5]):
    print(f"  {p}{' (M)' if m else ''}: 書き換え {c} 0-1だけ {b} 縮み {e} f={f:.3f} gate={g:.3f}  前処理の後 def {'x'.join(dp)} spls {'x'.join(sp)}{'  ← 入る' if g >= 0.5 else ''}")
print(f"\n入る問題 (gate ≥ 0.5): {len(gin)} {gin}")
print(f"入らないのに前処理の後の大きさが違う問題: {len(mism)} {mism}")
print(f"比べられない (取得の失敗・時間切れ・PRETIME の行なし): {len(bad)}")
for b in bad: print("  ", b)
