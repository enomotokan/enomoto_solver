"""run1.py の jsonl を表にする。使い方: python summ.py 生データ.jsonl [...]
nodecut の値の並び (MIP: nodecut の行の数の順):
 0 LP の数, 1 反復, 2 ObjectiveBound で止まった数, 3 その反復, 4 最適まで解いた数, 5 その反復,
 6 最適で出発の打ち切り値以上 (最適まで解いてから刈る), 7 その反復, 8 そのうち段階 B に入らなかった数, 9 その反復,
 10 そのうち段階 B に入ってからの反復, 11 段階 B だが一度も比べなかった数,
 12 最適で出発の打ち切り値未満・終わりに読み直した値以上 (読み直せば刈れた), 13 その反復, 14 途中で止められた反復の見積もり, 15 そのうち段階 B なし,
 16 求解中に読み直した値が下がった数, 17 下がった後に ObjectiveBound で止まった数, 18 その反復,
 19 ObjectiveBound のうち読み直した値なら早く止まった数, 20 その見積もり, 21 実行不能, 22 その反復, 23 出発の打ち切り値が無限,
 24 段階 B で比べた回数, 25 そのうち下界が -inf, 26 最適まで解いてから刈った LP のうち最後の下界が -inf, 27 その反復,
 28 -inf の列を飛ばした値なら出発の打ち切り値を越えていた数, 29 その見積もり
"""
import json, sys
from collections import defaultdict

rows = []
for fn in sys.argv[1:]:
    for l in open(fn):
        if l.strip():
            rows.append(json.loads(l))


def pct(a, b):
    return f"{100 * a / b:.1f}%" if b else "-"


print("| 問題 | 設定 | rep | 状態 | 秒 | ノード | 種類 | LP | 反復 | 反復/LP | 途中で停止 (反復) | 解いてから刈る (反復, 全体比) | 段階 A のまま | 下界 -inf で比べられず (反復) | 読み直せば刈れた (反復, 省ける見積もり) | 読み直しで下がった / その後停止 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    res = r["result"]
    for k, v in r["nodecut"].items():
        if len(v) < 30:
            continue
        print(f"| {r['problem']} | {r['cfg']} | {r['rep']} | {res.get('status')} | {res.get('time') or 0:.0f} | {res.get('nodes')} | {k} | {v[0]} | {v[1]} | {v[1] / max(v[0], 1):.0f} | "
              f"{v[2]} ({v[3]}) | {v[6]} ({v[7]}, {pct(v[7], v[1])}) | {v[8]} ({v[9]}) | {v[26]} ({v[27]}; 飛ばせば {v[28]} 件 {v[29]}) | {v[12]} ({v[13]}, {v[14]}) | {v[16]} / {v[17]} |")
