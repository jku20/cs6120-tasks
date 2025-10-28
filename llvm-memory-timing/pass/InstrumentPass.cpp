#include "llvm/IR/Module.h"
#include "llvm/Pass.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"

using namespace llvm;

namespace {

struct InstrumentPass : public PassInfoMixin<InstrumentPass> {
  PreservedAnalyses run(Module &M, ModuleAnalysisManager &AM) {
    for (auto &F : M) {
      LLVMContext &Ctx = F.getContext();
      FunctionCallee startFunc = F.getParent()->getOrInsertFunction(
          "start_timer", Type::getVoidTy(Ctx));
      FunctionCallee endFunc =
          F.getParent()->getOrInsertFunction("end_timer", Type::getVoidTy(Ctx));

      bool look_for_access = false;
      for (auto &B : F) {
        for (auto &I : B) {
          if (auto *op = dyn_cast<CallBase>(&I)) {
            auto Func = op->getCalledFunction();
            if (Func) {
              auto name = Func->getName();
              if (name == "llvm.var.annotation.p0.p0") {
                auto S =
                    cast<ConstantDataArray>(
                        cast<GlobalVariable>(op->getOperand(1))->getOperand(0))
                        ->getAsCString();
                if (S == "time") {
                  look_for_access = true;
                }
              }
            }
          }
          if (look_for_access) {
            if (auto *LI = dyn_cast<LoadInst>(&I)) {
              if (auto *GP =
                      dyn_cast<GetElementPtrInst>(LI->getPointerOperand())) {
                IRBuilder<> builder(LI);
                builder.CreateCall(startFunc);
                builder.SetInsertPoint(&B, ++builder.GetInsertPoint());
                builder.CreateCall(endFunc);
                look_for_access = false;
              }
            }
          }
        }
      }
    }

    return PreservedAnalyses::all();
  }
};
} // namespace

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return {.APIVersion = LLVM_PLUGIN_API_VERSION,
          .PluginName = "instrument pass",
          .PluginVersion = "v0.1",
          .RegisterPassBuilderCallbacks = [](PassBuilder &PB) {
            PB.registerOptimizerLastEPCallback([](ModulePassManager &MPM,
                                                  OptimizationLevel Level,
                                                  ThinOrFullLTOPhase LTOPhase) {
              MPM.addPass(InstrumentPass());
            });
          }};
}
