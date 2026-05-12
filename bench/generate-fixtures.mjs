import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { performance } from "node:perf_hooks";

const root = ".bench-fixtures";
const smallDir = join(root, "small");
const mediumDir = join(root, "medium-manifest");

async function writeMarkdownFixture(dir, count) {
  await mkdir(dir, { recursive: true });
  const started = performance.now();

  for (let index = 0; index < count; index += 1) {
    const id = String(index + 1).padStart(4, "0");
    await writeFile(
      join(dir, `note-${id}.md`),
      [
        `# Note ${id}`,
        "",
        "This is a synthetic Markdown note for o-note performance baselines.",
        "",
        `- index: ${index}`,
        "- format: markdown",
      ].join("\n"),
    );
  }

  return Math.round(performance.now() - started);
}

async function main() {
  const smallMs = await writeMarkdownFixture(smallDir, 100);
  await mkdir(mediumDir, { recursive: true });
  await writeFile(
    join(mediumDir, "manifest.json"),
    JSON.stringify({ notes: 10_000, htmlArtifacts: 1_000, generatedAt: new Date().toISOString() }, null, 2),
  );

  console.log(`small fixture: 100 markdown notes in ${smallMs}ms`);
  console.log("medium manifest: 10,000 notes / 1,000 HTML artifacts");
}

await main();
