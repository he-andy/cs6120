#include "llvm/Pass.h"
#include "llvm/Analysis/LoopPass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/IR/Dominators.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/BasicBlock.h"
#include "llvm/Analysis/LoopInfo.h"
#include "llvm/Analysis/ValueTracking.h"

using namespace llvm;

namespace
{
  struct LICM : public LoopPass
  {
    static char ID;
    LICM() : LoopPass(ID) {}
    virtual bool runOnLoop(Loop *L, LPPassManager &LPM);
    bool doFinalization() { return false; }

  private:
    LoopInfo *loopinfo;
    DominatorTree *domtree;
    BasicBlock *preheader;
    bool changed;
    void hoist(Instruction &I);
    bool safeToHoist(Instruction &I);
    Loop *loop;
  };
};

bool LICM::runOnLoop(Loop *L, LPPassManager &LPM)
{
  loopinfo = &getAnalysis<LoopInfo>();
  domtree = &getAnalysis<DominatorTree>();
  preheader = L->getLoopPreheader();
  loop = L;
  changed = false;
  // iterate over over the loop's basic blocks
  for (auto bb : L->getBlocks())
  {
    // ignore bb in subloops
    if (loopinfo->getLoopFor(bb) != L)
    {
      continue;
    }

    // iterate over the instructions in the basic block
    for (auto &I : *bb)
    {
      // if the instruction is safe to hoist and loop invariant, hoist
      if (safeToHoist(I) && L->isLoopInvariant(&dyn_cast<Value>(I)))
      {
        hoist(I);
      }
    }
  }
  return changed;
}

// An instruction is safe to hoist if either of the following is true:
// it has no side effects (exceptions/traps), or
// the basic block containing the instruction dominates all exit blocks for the loop.
bool LICM::safeToHoist(Instruction &I)
{
  if (isSafeToSpeculativelyExecute(&I))
  {
    return true;
  }
  SmallVector<BasicBlock *> ExitBlocks;
  loop->getUniqueExitBlocks(ExitBlocks);
  for (auto bb : ExitBlocks)
  {
    if (!domtree->dominates(I.getParent(), bb))
    {
      return false;
    }
  }

  return true;
}

void LICM::hoist(Instruction &I)
{
  I.moveBefore(preheader->getTerminator());
  changed = true;
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo()
{
  return {
      .APIVersion = LLVM_PLUGIN_API_VERSION,
      .PluginName = "LICM pass",
      .PluginVersion = "v0.1",
      .RegisterPassBuilderCallbacks = [](PassBuilder &PB)
      {
        PB.registerPipelineStartEPCallback(
            [](ModulePassManager &MPM, OptimizationLevel Level)
            {
              MPM.addPass(LICM());
            });
      }};
}
