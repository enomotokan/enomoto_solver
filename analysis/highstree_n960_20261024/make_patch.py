import sys,os
os.chdir(sys.argv[1])
def rep(p, old, new, count=1, nth=None):
    s=open(p).read()
    n=s.count(old)
    assert n>=1, (p, old[:60])
    if nth is None:
        assert n==count, (p, old[:60], n)
        s=s.replace(old,new)
    else:
        i=-1
        for _ in range(nth+1): i=s.index(old,i+1)
        s=s[:i]+new+s[i+len(old):]
    open(p,'w').write(s)

# ---- PrimalHeuristics.h: tag
rep('HighsPrimalHeuristics.h', ' public:\n', ' public:\n  std::string tr_tag;\n')
# ---- PrimalHeuristics.cpp
P='HighsPrimalHeuristics.cpp'
rep(P,'#include "mip/HighsPrimalHeuristics.h"\n','#include "mip/HighsPrimalHeuristics.h"\n#include "mip/HighsTrace.h"\n#include "lp_data/HighsModelUtils.h"\n')
rep(P,'''  mipsolver.profiling_->setSubMip(true);
  submipsolver.run();
''','''  mipsolver.profiling_->setSubMip(true);
  int tr_id = 0;
  double tr_t0 = 0;
  if (trOn()) {
    tr_id = ++trSubId();
    HighsInt nfix = 0, nint = 0;
    for (HighsInt c : mipsolver.mipdata_->integral_cols) {
      ++nint;
      if (submip.col_lower_[c] == submip.col_upper_[c]) ++nfix;
    }
    trPrint(int(mipsolver.submip_level),
            "SUBMIP_BEGIN id=%d tag={%s} fixrate=%.4f lp_rows=%d lp_cols=%d "
            "intfixed=%d/%d free_int=%d maxleaves=%d maxnodes=%d stall=%d "
            "cutoff=%.6g parent_nodes=%lld parent_lb=%.6g parent_ub=%.6g",
            tr_id, tr_tag.c_str(), fixingRate, int(submip.num_row_),
            int(submip.num_col_), int(nfix), int(nint), int(nint - nfix),
            int(maxleaves), int(maxnodes), int(stallnodes), worker.upper_limit,
            (long long)mipsolver.mipdata_->num_nodes,
            mipsolver.mipdata_->lower_bound, mipsolver.mipdata_->upper_bound);
    tr_t0 = trTime();
  }
  TrCtxGuard tr_guard("sub" + std::to_string(tr_id) + ":" +
                      tr_tag.substr(0, tr_tag.find(' ')));
  tr_tag.clear();
  submipsolver.run();
  if (trOn())
    trPrint(int(mipsolver.submip_level),
            "SUBMIP_END id=%d status=%s nodes=%lld secs=%.3f obj=%.6g "
            "sub_lb=%.6g nsol=%d",
            tr_id, utilModelStatusToString(submipsolver.modelstatus_).c_str(),
            (long long)submipsolver.node_count_, trTime() - tr_t0,
            submipsolver.solution_.empty() ? kHighsInf
                                           : submipsolver.solution_objective_,
            submipsolver.mipdata_ ? submipsolver.mipdata_->lower_bound
                                  : -kHighsInf,
            int(submipsolver.solution_.size() > 0));
''')
# rootReducedCost tag
rep(P,'''  if (fixingRate < 0.3) return;

  solveSubMip(''','''  if (fixingRate < 0.3) return;

  if (trOn())
    tr_tag = "RRC nodes=" + std::to_string(mipsolver.mipdata_->num_nodes) +
             " lurking=" + std::to_string(lurkingBounds.size()) +
             " cutoff_used=" + std::to_string(currCutoff) +
             " fixrate=" + std::to_string(fixingRate);
  solveSubMip(''')
TAGFN='''
static std::string trPointDesc(const HighsMipSolver& mipsolver,
                               const std::vector<double>& x) {
  double obj = 0;
  HighsInt nfrac = 0;
  for (HighsInt i = 0; i < mipsolver.model_->num_col_; ++i) {
    obj += mipsolver.model_->col_cost_[i] * x[i];
    if (mipsolver.model_->integrality_[i] != HighsVarType::kContinuous &&
        std::abs(x[i] - std::floor(x[i] + 0.5)) > mipsolver.mipdata_->feastol)
      ++nfrac;
  }
  return "obj=" + std::to_string(obj) + " nfrac=" + std::to_string(nfrac);
}

void HighsPrimalHeuristics::RENS('''
rep(P,'\nvoid HighsPrimalHeuristics::RENS(',TAGFN)
# RENS before removeObsoleteRows (first), RINS second
rep(P,'  heurlp.removeObsoleteRows(false);\n','''  if (trOn())
    tr_tag = std::string("RENS ") +
             (&tmp == &mipsolver.mipdata_->rootlpsol ? "where=rootlpsol"
                                                      : "where=nodelp") +
             " nodes=" + std::to_string(mipsolver.mipdata_->num_nodes) +
             " start{" + trPointDesc(mipsolver, tmp) + "} inc=" +
             std::to_string(mipsolver.mipdata_->upper_bound) +
             " lb=" + std::to_string(mipsolver.mipdata_->lower_bound) +
             " dive_depth=" + std::to_string(heur.getCurrentDepth()) +
             " target_fixrate=" + std::to_string(maxfixingrate) +
             " retry=" + std::to_string(nbacktracks);
  heurlp.removeObsoleteRows(false);
''',nth=0)
rep(P,'  heurlp.removeObsoleteRows(false);\n  HighsInt node_reduction_factor','''  if (trOn())
    tr_tag = std::string("RINS nodes=") +
             std::to_string(mipsolver.mipdata_->num_nodes) + " relax{" +
             trPointDesc(mipsolver, relaxationsol) + "} inc=" +
             std::to_string(mipsolver.mipdata_->upper_bound) +
             " lb=" + std::to_string(mipsolver.mipdata_->lower_bound) +
             " dive_depth=" + std::to_string(heur.getCurrentDepth()) +
             " target_fixrate=" + std::to_string(maxfixingrate) +
             " retry=" + std::to_string(nbacktracks);
  heurlp.removeObsoleteRows(false);
  HighsInt node_reduction_factor''',nth=1)
for k,name in ((0,'RENS'),(1,'RINS')):
    rep(P,'    heur.setMinReliable(0);\n','''    if (trOn())
      trPrint(int(mipsolver.submip_level),
              "%s_DIVEONLY nodes=%lld fixrate=%.4f",'''+f'"{name}"'+''',
              (long long)mipsolver.mipdata_->num_nodes, fixingrate);
    heur.setMinReliable(0);
''',nth=k) if k==0 else rep(P,'    heur.setMinReliable(0);\n    heur.solveDepthFirst(10);','''    if (trOn())
      trPrint(int(mipsolver.submip_level),
              "RINS_DIVEONLY nodes=%lld fixrate=%.4f",
              (long long)mipsolver.mipdata_->num_nodes, fixingrate);
    heur.setMinReliable(0);
    heur.solveDepthFirst(10);''',nth=1)

# ---- MipSolverData.cpp
D='HighsMipSolverData.cpp'
rep(D,'#include "mip/HighsMipSolverData.h"\n','#include "mip/HighsMipSolverData.h"\n#include "mip/HighsTrace.h"\n')
rep(D,'''void HighsMipSolverData::runSetup() {
  const HighsLp& model = *mipsolver.model_;
''','''void HighsMipSolverData::runSetup() {
  const HighsLp& model = *mipsolver.model_;
  if (trOn())
    trPrint(int(mipsolver.submip_level),
            "SETUP rows=%d cols=%d nnz=%d (orig rows=%d cols=%d)",
            int(model.num_row_), int(model.num_col_),
            int(model.a_matrix_.numNz()), int(mipsolver.orig_model_->num_row_),
            int(mipsolver.orig_model_->num_col_));
''')
rep(D,'''    double prev_upper_bound = upper_bound;

    upper_bound = solobj;
''','''    double prev_upper_bound = upper_bound;

    upper_bound = solobj;
    if (trOn())
      trPrint(int(mipsolver.submip_level),
              "INCUMBENT src=%s obj=%.6g obj+offset=%.6g lb=%.6g nodes=%lld "
              "leaves=%lld",
              solutionSourceToString(solution_source).c_str(), solobj,
              solobj + mipsolver.model_->offset_, lower_bound,
              (long long)num_nodes, (long long)num_leaves);
''')
rep(D,'''  status = evaluateRootLp(worker);
  if (status == HighsLpRelaxation::Status::kInfeasible) return true;

  const std::vector<double>& solvals =''','''  status = evaluateRootLp(worker);
  if (trOn())
    trPrint(int(mipsolver.submip_level),
            "ROOTSEP ncuts=%d lp_obj=%.6g lb=%.6g lp_rows=%d nfrac=%d "
            "status=%d",
            int(ncuts), getLp().getObjective(), lower_bound,
            int(getLp().numRows()), int(getLp().getFractionalIntegers().size()),
            int(status));
  if (status == HighsLpRelaxation::Status::kInfeasible) return true;

  const std::vector<double>& solvals =''')
rep(D,'''  rootlpsolobj = firstlpsolobj;
  removeFixedIndices();''','''  rootlpsolobj = firstlpsolobj;
  if (trOn())
    trPrint(int(mipsolver.submip_level),
            "ROOTLP first_obj=%.6g lp_rows=%d nfrac=%d lb=%.6g ub=%.6g",
            firstlpsolobj, int(getLp().numRows()),
            int(getLp().getFractionalIntegers().size()), lower_bound,
            upper_bound);
  removeFixedIndices();''')
rep(D,'''  rootlpsol = getLp().getLpSolver().getSolution().col_value;
  rootlpsolobj = getLp().getObjective();
''','''  rootlpsol = getLp().getLpSolver().getSolution().col_value;
  rootlpsolobj = getLp().getObjective();
  if (trOn())
    trPrint(int(mipsolver.submip_level),
            "ROOTCUTS_DONE rounds=%d lp_obj=%.6g lb=%.6g lp_rows=%d nfrac=%d "
            "ub=%.6g",
            int(nseparounds), rootlpsolobj, lower_bound,
            int(getLp().numRows()), int(getLp().getFractionalIntegers().size()),
            upper_bound);
''')
# ---- Search.cpp
S='HighsSearch.cpp'
rep(S,'#include "mip/HighsSearch.h"\n','#include "mip/HighsSearch.h"\n#include "mip/HighsTrace.h"\n')
rep(S,'''  if (!inheuristic && currnode.lower_bound > getOptimalityLimit())
    return NodeResult::kSubOptimal;
''','''  if (!inheuristic && currnode.lower_bound > getOptimalityLimit()) {
    if (trOn() && mipsolver.submip_level < trLevel())
      trPrint(int(mipsolver.submip_level),
              "NODE_PRUNE reason=suboptimal_early depth=%d lb=%.6g nodes=%lld",
              int(getCurrentDepth()), currnode.lower_bound,
              (long long)(mipsolver.mipdata_->num_nodes + nnodes));
    return NodeResult::kSubOptimal;
  }
''')
rep(S,'''      result = NodeResult::kSubOptimal;
      addBoundExceedingConflict();
    }
  }

  return result;''','''      result = NodeResult::kSubOptimal;
      addBoundExceedingConflict();
    }
  }
  if (trOn() && !inheuristic && mipsolver.submip_level < trLevel()) {
    static const char* rn[] = {"bound", "domain_infeasible", "lp_infeasible",
                               "branched", "suboptimal", "open"};
    if (result != NodeResult::kOpen)
      trPrint(int(mipsolver.submip_level),
              "NODE_PRUNE reason=%s depth=%d lb=%.6g lp_obj=%.6g nodes=%lld "
              "glb=%.6g ub=%.6g",
              rn[int(result)], int(getCurrentDepth()), currnode.lower_bound,
              lp ? lp->getObjective() : -kHighsInf,
              (long long)(mipsolver.mipdata_->num_nodes + nnodes),
              mipsolver.mipdata_->lower_bound, mipsolver.mipdata_->upper_bound);
    else
      trPrint(int(mipsolver.submip_level),
              "NODE_EVAL depth=%d lb=%.6g lp_obj=%.6g nfrac=%d nodes=%lld",
              int(getCurrentDepth()), currnode.lower_bound,
              lp ? lp->getObjective() : -kHighsInf,
              lp ? int(lp->getFractionalIntegers().size()) : -1,
              (long long)(mipsolver.mipdata_->num_nodes + nnodes));
  }

  return result;''')
rep(S,'''  localdom.changeBound(currnode.branchingdecision);
  currnode.opensubtrees = 1;''','''  if (trOn() && !inheuristic && mipsolver.submip_level < trLevel())
    trPrint(int(mipsolver.submip_level),
            "BRANCH col=%d dir=%s val=%g point=%.4f depth=%d lb=%.6g "
            "childlb=%.6g otherlb=%.6g nodes=%lld",
            int(currnode.branchingdecision.column),
            currnode.branchingdecision.boundtype == HighsBoundType::kLower
                ? "up"
                : "down",
            currnode.branchingdecision.boundval, currnode.branching_point,
            int(getCurrentDepth()), currnode.lower_bound, childLb,
            currnode.other_child_lb,
            (long long)(mipsolver.mipdata_->num_nodes + nnodes));
  localdom.changeBound(currnode.branchingdecision);
  currnode.opensubtrees = 1;''')
print("patched")
