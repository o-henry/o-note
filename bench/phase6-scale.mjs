import { stat, writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { performance } from "node:perf_hooks";

const started = performance.now();
const mediumManifestPath = join(".bench-fixtures", "medium-manifest", "manifest.json");

async function exists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

const mediumManifestExists = await exists(mediumManifestPath);
const report = {
  measuredAt: new Date().toISOString(),
  budgets: {
    searchWarm10kMs: 100,
    htmlPreviewMountMs: 250,
    warmLaunchMs: 700,
  },
  gates: {
    mediumManifestExists,
    bundleSizeWarnKb: 500,
    sqliteFtsDecision: "keep-sqlite-fts5-until-measured-miss",
  },
  harnessOverheadMs: Math.round(performance.now() - started),
  status: mediumManifestExists ? "ok" : "warn",
};

await mkdir("docs/performance", { recursive: true });
await writeFile("docs/performance/phase-06-baseline.json", `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
