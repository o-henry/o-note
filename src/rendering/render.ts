import DOMPurify from "dompurify";
import MarkdownIt from "markdown-it";

const markdown = new MarkdownIt({
  breaks: true,
  html: false,
  linkify: true,
  typographer: true,
});

const markdownCache = new Map<string, string>();

export function renderMarkdown(source: string) {
  const cached = markdownCache.get(source);

  if (cached) {
    return cached;
  }

  const rendered = markdown.render(source);
  const sanitized = DOMPurify.sanitize(rendered, {
    USE_PROFILES: { html: true },
  });
  markdownCache.set(source, sanitized);

  return sanitized;
}

export function createSandboxDocument(source: string) {
  return [
    "<!doctype html>",
    '<html lang="en">',
    "<head>",
    '<meta charset="utf-8" />',
    '<meta name="viewport" content="width=device-width, initial-scale=1" />',
    '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; img-src data: blob:; style-src \'unsafe-inline\'; script-src \'unsafe-inline\'; connect-src \'none\'; form-action \'none\'; base-uri \'none\'" />',
    "<style>",
    "html{color-scheme:light dark;font-family:Inter,system-ui,sans-serif;background:#fff;color:#111}",
    "body{margin:0;padding:24px;line-height:1.5}",
    ".o-note-static-fallback{border:1px solid #bbb;padding:10px;margin-bottom:16px;font:12px ui-monospace,monospace;text-transform:uppercase;color:#555}",
    "</style>",
    "</head>",
    "<body>",
    '<div class="o-note-static-fallback">Sandboxed artifact: scripts may only post allowlisted messages; network, forms, and local file access are disabled.</div>',
    source,
    "</body>",
    "</html>",
  ].join("");
}

export function isAllowedArtifactMessage(value: unknown) {
  if (!isRecord(value)) {
    return false;
  }

  return (
    value.version === 1 &&
    typeof value.noteId === "string" &&
    typeof value.artifactId === "string" &&
    ["copy_text", "copy_markdown", "copy_json", "copy_diff", "export_html"].includes(
      String(value.command),
    ) &&
    (value.payload === undefined || typeof value.payload === "string")
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
