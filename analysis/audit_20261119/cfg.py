"""AUDIT の確かめの設定 (analysis/audit_20261119/README.md)。def = 今の既定 (環境変数なし)。各案は疑い「高」の既定を 1 つずつ外す。
切った形が採用前の動きになることは、採用の節 (docs/improvement_history.md) とソースのコメントで確かめた:
- sw0: `ENOMOTO_T_MIP_SUBWAIT=0`。2026-11-18 の節「`=0` で従来どおり使わない」。src/mip/solver.rs の `sw_k` が 0 なら打ち切りの枝に入らない。
- rins: `ENOMOTO_MIP_S4_RINSRATE=0`・`ENOMOTO_MIP_S4_BGZERO=0`。2026-11-06 の節「それぞれ戻し方 `=0`」。`env_def!` は `0` で None になり、
  RINS の固定率の下限 (heuristics.rs・heur_subfix.rs) と裏のカットの早めの打ち切り (solver.rs の `zero_r`) の枝に入らない (0.1.9 前の 11-06 前の形)。
- qiu: `ENOMOTO_MIP_QIU_FIND_STOP_GAP=0`。2026-11-04 の節・solver.rs のコメント「`=0` で以前の形」。`only_fj` が `inc_only_fj` だけになる。
- cs2: `ENOMOTO_MIP_CUT_LP_CAP_STOP=0`。2026-11-01 の節「`flat` ... 戻し方 `0`」。separation.rs の `lpcap_stop` が None になる。
  cs2 のほかの部品 (CUT_LP_WORK・CUT_LP_WORK_PREV・CUT_CAP_FROM_LOOP) は反復で数える側で運を減らすので残す。"""
CFGS = {
    "def": {},
    "sw0": {"ENOMOTO_T_MIP_SUBWAIT": "0"},
    "rins": {"ENOMOTO_MIP_S4_RINSRATE": "0", "ENOMOTO_MIP_S4_BGZERO": "0"},
    "qiu": {"ENOMOTO_MIP_QIU_FIND_STOP_GAP": "0"},
    "cs2": {"ENOMOTO_MIP_CUT_LP_CAP_STOP": "0"},
}
