"""当方の MPS 読み込みの結果 (変数の境界・整数性・目的・行の境界・非零) の指紋を、問題ごとに 1 行で出す (読めなければ理由)。
直す前の wheel と直した後の wheel を PYTHONPATH で切り替えて回し、出力を比べる: python readcmp.py ファイル... > 出力.jsonl"""
import gzip, hashlib, json, sys
from enomoto_solver import _core
for p in sys.argv[1:]:
    opener = gzip.open if p.endswith(".gz") else open
    with opener(p, "rt", encoding="latin-1") as fh:
        text = fh.read()
    m = _core.PyModel()
    try:
        _, names, _ = m.read_mps(text)
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"file": p, "error": str(e)}), flush=True); continue
    h = hashlib.sha256()
    for part in (m.variable_bounds(), m.objective_terms(), names, m.n_constraints(), m.n_nonzeros()):
        h.update(repr(part).encode())
    print(json.dumps({"file": p, "n_constraints": m.n_constraints(), "nnz": m.n_nonzeros(), "hash": h.hexdigest()}), flush=True)
