"""densecut の診断だけの一覧: python diag.py RES.jsonl CFG。問題ごと (スレッドの和) に、ノードで足したカットの本数・非零/本・元の行の平均との比・
根の後のカットの行の非零 / 元の行の非零 (ノードの LP 1 回あたり)・ノードの LP 1 回の ms・ノードの分離の秒・伸びの平均・密判定の回数。"""
import json, sys
sys.path.insert(0, __import__("os").path.dirname(__file__))
from summ_lib import agg
res, cfg = sys.argv[1], sys.argv[2]
print(f"{'問題':24s} 種 状態  秒     カット 非零/本 元の行 比率 後の行/LP 後の非零/元 ms/LP  LP秒   分離秒 伸び平均 測った回 密判定 ノード")
for l in open(res):
    r = json.loads(l)
    if r["cfg"] != cfg: continue
    a = agg(r); lps = max(a["lps"], 1); x = r["result"]
    print(f"{r['problem']:24s} {r['seed']} {x.get('status','?')[:5]:5s} {x.get('time') or 0:6.1f} {a['ncuts']:6.0f} {a['nnz']/max(a['ncuts'],1):7.1f} {a['rowavg']:6.1f} {a['nnz']/max(a['ncuts'],1)/max(a['rowavg'],1e-9):5.1f} {a['post']/lps:7.1f} {a['postnz']/lps/max(a['orignnz'],1):6.3f} {a['lpsecs']/lps*1000:7.2f} {a['lpsecs']:6.1f} {a['cutsecs']:6.2f} {a['gsum']/max(a['gn'],1):7.3f} {a['gn']:6.0f} {a['dh']:6.0f} {int(a['nodes'])}")
