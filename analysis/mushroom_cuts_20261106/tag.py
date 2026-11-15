import re,sys
D="/tmp/HiGHS/highs/mip/"
def sub(f, old, new, cnt=1):
    s=open(D+f).read()
    assert s.count(old)>=1, (f, old[:60])
    s=s.replace(old,new,cnt); open(D+f,"w").write(s)
# globals in HighsCutPool.cpp
sub("HighsCutPool.cpp", '#include "mip/HighsCutPool.h"', '''#include "mip/HighsCutPool.h"
#include <cstdio>
#include <cstdlib>
#include <mutex>
#include <string>
thread_local const char* g_hsrc = "other";
thread_local int g_hmeth = 0;
thread_local std::string g_hagg;
static std::mutex g_hpoolMu;
static FILE* hpoolF() {
  static FILE* f = getenv("HIGHS_POOL_TAG") ? fopen(getenv("HIGHS_POOL_TAG"), "w") : nullptr;
  return f;
}
void hpoolLog(const char* line) {
  FILE* f = hpoolF(); if (!f) return;
  std::lock_guard<std::mutex> g(g_hpoolMu); fputs(line, f); fflush(f);
}''')
sub("HighsCutPool.cpp", '''  rhs_[rowindex] = rhs;
  ages_[rowindex]''', '''  rhs_[rowindex] = rhs;
  if (hpoolF()) {
    std::string s; char b[64];
    snprintf(b, sizeof b, "P %d %p %d %s %d %d %.17g %d |", (int)mipsolver.submip, (void*)this,
             (int)rowindex, g_hsrc, g_hmeth, (int)isConflict, rhs, (int)Rlen);
    s += b;
    for (HighsInt i = 0; i != Rlen; ++i) { snprintf(b, sizeof b, " %d %.12g", (int)Rindex[i], Rvalue[i]); s += b; }
    s += " |"; s += g_hagg; s += "\\n";
    hpoolLog(s.c_str());
  }
  ages_[rowindex]''')
# aggregator: record rows
sub("HighsLpAggregator.cpp", '''void HighsLpAggregator::addRow(HighsInt row, double weight) {
  HighsInt len;''', '''#include <string>
extern thread_local std::string g_hagg;
void HighsLpAggregator::addRow(HighsInt row, double weight) {
  { char b[64]; snprintf(b, sizeof b, " %d:%.10g", (int)row, weight); g_hagg += b; }
  HighsInt len;''')
sub("HighsLpAggregator.cpp", "void HighsLpAggregator::clear() { vectorsum.clear(); }",
    "void HighsLpAggregator::clear() { vectorsum.clear(); g_hagg.clear(); }")
# methods in tryGenerateCut
s=open(D+"HighsCutGeneration.cpp").read()
s=s.replace('extern FILE* g_gcTraceF;','extern FILE* g_gcTraceF;\nextern thread_local int g_hmeth;\nextern thread_local const char* g_hsrc;\nextern thread_local std::string g_hagg;',1)
s=s.replace('''    return cmirCutGenerationHeuristic(minEfficacy, onlyInitialCMIRScale);
  }

  // 0. Save data''','''    g_hmeth = 1;
    return cmirCutGenerationHeuristic(minEfficacy, onlyInitialCMIRScale);
  }

  // 0. Save data''',1)
s=s.replace('''    // take the cmir cut as it is better
''','''    // take the cmir cut as it is better
    g_hmeth = 1;
''',1)
s=s.replace('''  } else if (success) {
''','''  } else if (success) {
    g_hmeth = (!hasContinuous && !hasGeneralInts) ? 2 : hasGeneralInts ? 3 : 4;
''',1)
open(D+"HighsCutGeneration.cpp","w").write(s)
