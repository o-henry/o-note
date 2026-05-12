import { performance } from "node:perf_hooks";

const started = performance.now();

const baseline = {
  measuredAt: new Date().toISOString(),
  shellBudgetMs: 1_500,
  warmBudgetMs: 700,
  harnessOverheadMs: Math.round(performance.now() - started),
  note: "Phase 0 placeholder records the budget before browser-level startup probes exist.",
};

console.log(JSON.stringify(baseline, null, 2));
