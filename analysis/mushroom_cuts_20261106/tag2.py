D="/tmp/HiGHS/highs/mip/"
def sub(f, old, new, cnt=1):
    s=open(D+f).read()
    assert s.count(old)>=1, (f, old[:60])
    s=s.replace(old,new,cnt); open(D+f,"w").write(s)
sub("HighsCutGeneration.cpp", '#include <cstdio>', '#include <cstdio>\n#include <string>')
sub("HighsCutGeneration.cpp", '''                                          double& proofrhs) {
  this->inds = proofinds.data();''', '''                                          double& proofrhs) {
  const char* hsrcSave = g_hsrc;
  struct HR { const char* s; ~HR() { g_hsrc = s; } } hr{hsrcSave};
  g_hsrc = (hsrcSave && hsrcSave[0] == 'c' && hsrcSave[1] == 'f') ? hsrcSave : "cf-other";
  g_hmeth = 5; g_hagg.clear();
  this->inds = proofinds.data();''')
# separation tags
s=open(D+"HighsSeparation.cpp").read()
s=s.replace('#include <cstdio>','#include <cstdio>\n#include <string>\nextern thread_local const char* g_hsrc;\nextern thread_local int g_hmeth;\nextern thread_local std::string g_hagg;',1)
s=s.replace('  if (!getenv("HIGHS_NO_IMPLB")) mipdata.implications','  g_hsrc = "impl"; g_hmeth = 6; g_hagg.clear();\n  if (!getenv("HIGHS_NO_IMPLB")) mipdata.implications',1)
s=s.replace('  if (!getenv("HIGHS_NO_CLQ")) mipdata.cliquetable','  g_hsrc = "clq"; g_hmeth = 7; g_hagg.clear();\n  if (!getenv("HIGHS_NO_CLQ")) mipdata.cliquetable',1)
s=s.replace('    separator->run(*lp, lpAggregator, transLp, mipworker_.getCutPool());','    { static const char* nm[] = {"tab", "path", "modk", "x3", "x4"}; g_hsrc = nm[&separator - &separators[0]]; g_hmeth = 0; g_hagg.clear(); }\n    separator->run(*lp, lpAggregator, transLp, mipworker_.getCutPool());\n    g_hsrc = "other";',1)
open(D+"HighsSeparation.cpp","w").write(s)
# conflict callers
for f,tag in [("HighsSearch.cpp","cf-search"),("HighsPrimalHeuristics.cpp","cf-heur")]:
    s=open(D+f).read()
    s="#include <string>\nextern thread_local const char* g_hsrc;\n"+s
    s=s.replace("cutGen.generateConflict(", 'g_hsrc = "%s"; cutGen.generateConflict('%tag)
    open(D+f,"w").write(s)
# LP adds
sub("HighsLpRelaxation.cpp", '''void HighsLpRelaxation::addCuts(HighsCutSet& cutset) {
  HighsInt numcuts = cutset.numCuts();''', '''void hpoolLog(const char* line);
void HighsLpRelaxation::addCuts(HighsCutSet& cutset) {
  HighsInt numcuts = cutset.numCuts();
  for (HighsInt i = 0; i < numcuts; ++i) {
    char b[96]; snprintf(b, sizeof b, "L %d %d %d %d %.9g\\n", (int)mipsolver.submip, (int)cutset.cutindices[i], (int)cutset.cutpools[i], (int)lpsolver.getLp().num_row_, getObjective());
    hpoolLog(b);
  }''')
