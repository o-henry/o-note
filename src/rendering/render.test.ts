import { describe, expect, it } from "vitest";
import {
  createSandboxDocument,
  isAllowedArtifactMessage,
  renderMarkdown,
} from "./render";

describe("rendering", () => {
  it("sanitizes script injection from Markdown", () => {
    const rendered = renderMarkdown("# Hello\n\n<script>alert('x')</script>");

    expect(rendered).toContain("<h1>Hello</h1>");
    expect(rendered).not.toContain("<script>");
  });

  it("wraps HTML artifacts in an isolated interactive sandbox document", () => {
    const document = createSandboxDocument("<h1>Artifact</h1><script>alert('x')</script>");

    expect(document).toContain("Content-Security-Policy");
    expect(document).toContain("script-src 'unsafe-inline'");
    expect(document).toContain("Sandboxed artifact");
    expect(document).toContain("<h1>Artifact</h1>");
  });

  it("validates artifact bridge messages with an allowlist", () => {
    expect(
      isAllowedArtifactMessage({
        version: 1,
        noteId: "n-1",
        artifactId: "a-1",
        command: "copy_markdown",
        payload: "# Export",
      }),
    ).toBe(true);
    expect(
      isAllowedArtifactMessage({
        version: 1,
        noteId: "n-1",
        artifactId: "a-1",
        command: "run_shell",
      }),
    ).toBe(false);
  });
});
