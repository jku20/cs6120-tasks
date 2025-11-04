#include "llvm/Pass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Transforms/Scalar/LoopRotation.h"
#include "llvm/Transforms/Utils/Mem2Reg.h"
#include <iterator>

using namespace llvm;

namespace {
struct LICMPass : public PassInfoMixin<LICMPass> {
  PreservedAnalyses run(Function &F, FunctionAnalysisManager &AM) {
    if (F.isDeclaration())
      return PreservedAnalyses::all();

    FunctionPassManager CanonicalizeLoops;
    CanonicalizeLoops.addPass(LoopSimplifyPass());
    CanonicalizeLoops.addPass(LCSSAPass());
    CanonicalizeLoops.addPass(
        createFunctionToLoopPassAdaptor(LoopRotatePass()));
    CanonicalizeLoops.run(F, AM);
    auto &LI = AM.getResult<LoopAnalysis>(F);
    auto &SE = AM.getResult<ScalarEvolutionAnalysis>(F);
    auto Loops = LI.getLoopsInPreorder();
    for (auto i = Loops.rbegin(), End = Loops.rend(); i != End; i++) {
      auto L = *i;
      // Loop simplify pass can fail when entered by indirectbr due to
      // critical edges not being able to be split.
      if (!L->isLoopSimplifyForm())
        continue;

      SmallVector<Instruction *> Invariant;
      for (auto B : L->getBlocks()) {
        for (auto &I : *B) {
          bool innermost = true;
          for (auto Inner : L->getLoopsInPreorder()) {
            if (Inner == L)
              continue;
            if (Inner->contains(&I)) {
              innermost = false;
              break;
            }
          }
          if (!innermost)
            continue;
          auto S = SE.getSCEV(&I);
          // Check that this is the innermost loop containing the variable.
          if (SE.isLoopInvariant(S, L)) {
            Invariant.push_back(&I);
          }
        }
      }

      auto H = L->getHeader();
      auto PH = H->getPrevNode();
      errs() << "preheader: " << *PH << "\n";
      for (auto I : Invariant) {
        if (auto P = dyn_cast<PHINode>(I)) {
          // Pass, this is annoying to do
        } else {
          errs() << "inserting: " << *I << "\n";
          I->moveBefore(PH->getTerminator()->getIterator());
        }
      }
    }

    return PreservedAnalyses::none();
  }
};
} // namespace

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return {.APIVersion = LLVM_PLUGIN_API_VERSION,
          .PluginName = "my-licm",
          .PluginVersion = "v0.1",
          .RegisterPassBuilderCallbacks = [](PassBuilder &PB) {
            PB.registerOptimizerEarlyEPCallback([](ModulePassManager &MPM,
                                                   OptimizationLevel,
                                                   ThinOrFullLTOPhase) {
              MPM.addPass(createModuleToFunctionPassAdaptor(PromotePass()));
              MPM.addPass(createModuleToFunctionPassAdaptor(LICMPass()));
            });
            PB.registerPipelineParsingCallback(
                [](StringRef Name, ModulePassManager &MPM,
                   ArrayRef<llvm::PassBuilder::PipelineElement>) {
                  if (Name == "my-licm") {
                    MPM.addPass(createModuleToFunctionPassAdaptor(LICMPass()));
                    return true;
                  }
                  return false;
                });
          }};
}
