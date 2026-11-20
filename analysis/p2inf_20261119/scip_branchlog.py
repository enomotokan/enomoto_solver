"""P2-INF: SCIP で 1 問を解き、ノードに入るたびに、そのノードを作った分枝 (変数・向き・値)・深さ・ノード番号 (最初の 60 ノードは局所的に固定された列の数も) を記録する。
python scip_branchlog.py <mps> <秒> <出力 (TSV)> [param=value ...]。当方の前処理後の問題 (列 C<j>) を前処理なしで解き、当方の EBR の行と並べる用。"""
import sys
from pyscipopt import Model, Eventhdlr, SCIP_EVENTTYPE
class H(Eventhdlr):
    def __init__(self, out): self.out = out
    def eventinit(self): self.model.catchEvent(SCIP_EVENTTYPE.NODEFOCUSED, self)
    def eventexit(self): self.model.dropEvent(SCIP_EVENTTYPE.NODEFOCUSED, self)
    def eventexec(self, event):
        nd = self.model.getCurrentNode()
        # 最初の 60 ノードだけ、ノードに入った時点 (伝播の前) で局所的に固定されている列の数も出す (重いので数を絞る)
        nfix = -1
        if self.model.getNNodes() <= 60:
            nfix = sum(1 for v in self.model.getVars(transformed=True) if v.getLbLocal() > v.getUbLocal() - 0.5)
        par = nd.getParent()
        br = nd.getParentBranchings()
        if br is None:
            return
        vs, bs, ts = br
        for v, b, t in zip(vs, bs, ts):
            self.out.write(f"{nd.getNumber()}\t{par.getNumber() if par else 0}\t{nd.getDepth()}\t{v.name}\t{'up' if t == 0 else 'down'}\t{b}\t{nfix}\n")
m = Model(); m.hideOutput()
m.readProblem(sys.argv[1]); m.setParam("limits/time", float(sys.argv[2])); m.setParam("parallel/maxnthreads", 1)
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1); v = {"TRUE": True, "FALSE": False}.get(v, v)
    try: v = int(v)
    except (ValueError, TypeError): pass
    m.setParam(k, v)
with open(sys.argv[3], "w") as f:
    m.includeEventhdlr(H(f), "brlog", "branch log")
    m.optimize()
print(m.getStatus(), m.getNNodes(), round(m.getSolvingTime(), 1))
