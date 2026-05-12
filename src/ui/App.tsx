import { useCallback, useEffect, useMemo, useState } from "react";
import type { KeyboardEvent } from "react";
import { createSandboxDocument, isAllowedArtifactMessage, renderMarkdown } from "../rendering/render";
import { notesApi } from "../services/notes";
import type {
  HealthRow,
  IndexHealth,
  NoteDetail,
  NoteFormat,
  NoteSummary,
  RenderMode,
  SearchResult,
} from "../shared/domain";

type ArtifactCategory = "planning" | "review" | "research" | "prototype";

type Annotation = {
  noteId: string;
  text: string;
  status: "open" | "resolved";
};

export function App() {
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [activeNote, setActiveNote] = useState<NoteDetail | null>(null);
  const [draft, setDraft] = useState("");
  const [previewSource, setPreviewSource] = useState("");
  const [renderMode, setRenderMode] = useState<RenderMode>("split");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchFormat, setSearchFormat] = useState<NoteFormat | "all">("all");
  const [selectedSearchIndex, setSelectedSearchIndex] = useState(0);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [indexHealth, setIndexHealth] = useState<IndexHealth>({ pending: 0, indexed: 0, failed: 0 });
  const [importPathValue, setImportPathValue] = useState("");
  const [exportPathValue, setExportPathValue] = useState("");
  const [backupPathValue, setBackupPathValue] = useState("");
  const [annotationDraft, setAnnotationDraft] = useState("");
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [saveState, setSaveState] = useState("Idle");
  const [transferState, setTransferState] = useState("Idle");
  const [reliabilityState, setReliabilityState] = useState("Unchecked");
  const visibleNotes = useMemo(() => notes.slice(0, 100), [notes]);
  const renderedMarkdown = useMemo(() => {
    if (activeNote?.format !== "markdown") return "";
    return renderMarkdown(previewSource);
  }, [activeNote?.format, previewSource]);
  const sandboxDocument = useMemo(() => {
    if (activeNote?.format !== "html") return "";
    return createSandboxDocument(previewSource);
  }, [activeNote?.format, previewSource]);

  const exportSource = useCallback(() => {
    if (!activeNote) return;
    const extension = activeNote.format === "html" ? "html" : "md";
    const blob = new Blob([draft], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${activeNote.title}.${extension}`;
    anchor.click();
    URL.revokeObjectURL(url);
    setSaveState("Exported");
  }, [activeNote, draft]);

  const refreshNotes = useCallback(async (nextActiveId?: string | null) => {
    const summaries = await notesApi.listNotes({ limit: 100, offset: 0 });
    setNotes(summaries);

    if (!nextActiveId && summaries[0]) {
      setActiveId(summaries[0].id);
    }
  }, []);

  useEffect(() => {
    void refreshNotes();
  }, [refreshNotes]);

  useEffect(() => {
    if (!activeId) return;

    let cancelled = false;
    notesApi.getNote(activeId).then((note) => {
      if (cancelled || !note) return;
      setActiveNote(note);
      setDraft(note.content);
      setPreviewSource(note.content);
      setSaveState("Loaded");
    });

    return () => {
      cancelled = true;
    };
  }, [activeId]);

  useEffect(() => {
    if (!activeNote || draft === activeNote.content) return;

    setSaveState("Queued");
    const handle = window.setTimeout(() => {
      setPreviewSource(draft);
      setSaveState("Saving");
      notesApi
        .updateNote({ id: activeNote.id, content: draft })
        .then((updated) => {
          setActiveNote(updated);
          setSaveState("Saved");
          return refreshNotes(updated.id);
        })
        .catch(() => setSaveState("Save failed"));
    }, 500);

    return () => window.clearTimeout(handle);
  }, [activeNote, draft, refreshNotes]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      notesApi
        .searchNotes({
          query: searchQuery,
          limit: 20,
          format: searchFormat === "all" ? undefined : searchFormat,
        })
        .then((results) => {
          setSearchResults(results);
          setSelectedSearchIndex(0);
        })
        .catch(() => setSearchResults([]));
    }, 120);

    return () => window.clearTimeout(handle);
  }, [searchFormat, searchQuery]);

  useEffect(() => {
    let cancelled = false;
    const refreshIndexHealth = () => {
      notesApi
        .indexHealth()
        .then((health) => {
          if (!cancelled) setIndexHealth(health);
        })
        .catch(() => {});
    };
    refreshIndexHealth();
    const handle = window.setInterval(refreshIndexHealth, 2_000);

    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, []);

  useEffect(() => {
    const handle = async (event: MessageEvent) => {
      if (!event.data || typeof event.data !== "object") return;
      if (!isAllowedArtifactMessage(event.data)) {
        setSaveState("Bridge blocked");
        return;
      }

      const message = event.data as { command: string; payload?: string };
      setSaveState("Bridge allowed");
      if (message.command.startsWith("copy_") && message.payload) {
        await navigator.clipboard?.writeText(message.payload);
        setSaveState("Bridge copied");
      }
      if (message.command === "export_html") {
        exportSource();
      }
    };

    window.addEventListener("message", handle);

    return () => window.removeEventListener("message", handle);
  }, [exportSource]);

  async function createNote(format: NoteFormat) {
    const created = await notesApi.createNote({
      title: format === "html" ? "Untitled HTML artifact" : "Untitled note",
      format,
      content: format === "html" ? "<section>\n  <h1>Untitled</h1>\n</section>" : "# Untitled\n",
    });
    await refreshNotes(created.id);
    setActiveId(created.id);
  }

  async function createArtifactFromTemplate(category: ArtifactCategory) {
    const created = await notesApi.createNote({
      title: `${categoryLabel(category)} artifact`,
      format: "html",
      content: artifactTemplate(category),
    });
    await refreshNotes(created.id);
    setActiveId(created.id);
    setRenderMode("preview");
  }

  async function renameActiveNote() {
    if (!activeNote) return;
    const title = window.prompt("Rename note", activeNote.title);
    if (title === null) return;
    const renamed = await notesApi.renameNote(activeNote.id, title);
    setActiveNote({ ...activeNote, title: renamed.title, updatedAt: renamed.updatedAt });
    await refreshNotes(activeNote.id);
  }

  async function deleteActiveNote() {
    if (!activeNote) return;
    await notesApi.deleteNote(activeNote.id);
    setActiveNote(null);
    setDraft("");
    setActiveId(null);
    await refreshNotes(null);
  }

  async function copySource() {
    if (!activeNote) return;
    await navigator.clipboard?.writeText(draft);
    setSaveState("Copied");
  }

  function openSearchResult(result: SearchResult) {
    setActiveId(result.id);
    setSearchQuery("");
    setSearchResults([]);
  }

  async function importFromPath() {
    if (!importPathValue.trim()) return;
    setTransferState("Importing");
    const report = await notesApi.importPath({ path: importPathValue.trim() });
    setTransferState(`${report.importedNotes} notes / ${report.importedAttachments} files`);
    await refreshNotes();
  }

  async function exportActiveNote(bundle: boolean) {
    if (!activeNote || !exportPathValue.trim()) return;
    setTransferState("Exporting");
    const report = await notesApi.exportNote({
      id: activeNote.id,
      path: exportPathValue.trim(),
      bundle,
    });
    setTransferState(`${report.filesWritten} files written`);
  }

  function saveAnnotation() {
    if (!activeNote || !annotationDraft.trim()) return;
    setAnnotations((items) => [
      { noteId: activeNote.id, text: annotationDraft.trim(), status: "open" },
      ...items,
    ]);
    setAnnotationDraft("");
    setSaveState("Annotated");
  }

  async function runIntegrityCheck() {
    const report = await notesApi.databaseIntegrity();
    setReliabilityState(`${report.status}: ${report.detail}`);
  }

  async function backupToPath() {
    if (!backupPathValue.trim()) return;
    const report = await notesApi.backupDatabase({ path: backupPathValue.trim() });
    setReliabilityState(`backup: ${report.detail}`);
  }

  async function repairIndex() {
    const report = await notesApi.repairSearchIndex();
    setReliabilityState(report.detail);
    await notesApi.indexHealth().then(setIndexHealth).catch(() => {});
  }

  function handleSearchKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelectedSearchIndex((index) => Math.min(index + 1, Math.max(searchResults.length - 1, 0)));
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelectedSearchIndex((index) => Math.max(index - 1, 0));
    }

    if (event.key === "Enter" && searchResults[selectedSearchIndex]) {
      event.preventDefault();
      openSearchResult(searchResults[selectedSearchIndex]);
    }
  }

  const healthRows: HealthRow[] = [
    { label: "Shell", value: "Ready", tone: "steady" },
    {
      label: "Index",
      value: indexHealth.pending > 0 ? `${indexHealth.pending} queued` : "Idle",
      tone: indexHealth.failed > 0 ? "warn" : indexHealth.pending > 0 ? "active" : "steady",
    },
    { label: "HTML", value: "Sandboxed", tone: "active" },
    { label: "Storage", value: "Local", tone: "steady" },
  ];

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Workspace navigation">
        <div className="brand-block">
          <span className="eyebrow">LOCAL NOTES</span>
          <strong>o-note</strong>
        </div>
        <nav className="nav-stack" aria-label="Primary">
          <a className="nav-item is-active" href="#notes">
            Notes
          </a>
          <a className="nav-item" href="#artifacts">
            Artifacts
          </a>
          <a className="nav-item" href="#sources">
            Sources
          </a>
          <a className="nav-item" href="#index">
            Index
          </a>
        </nav>
        <div className="action-grid" aria-label="Create notes">
          <button type="button" onClick={() => void createNote("markdown")}>
            New MD
          </button>
          <button type="button" onClick={() => void createNote("html")}>
            New HTML
          </button>
        </div>
        <section className="sidebar-panel" aria-labelledby="templates-heading">
          <h2 id="templates-heading">Templates</h2>
          <div className="template-grid">
            {(["planning", "review", "research", "prototype"] as const).map((category) => (
              <button
                key={category}
                onClick={() => void createArtifactFromTemplate(category)}
                type="button"
              >
                {categoryLabel(category)}
              </button>
            ))}
          </div>
        </section>
        <section className="sidebar-panel" aria-labelledby="search-heading">
          <h2 id="search-heading">Search</h2>
          <input
            aria-label="Search notes"
            className="search-input"
            onChange={(event) => setSearchQuery(event.currentTarget.value)}
            onKeyDown={handleSearchKeyDown}
            placeholder="/ search"
            value={searchQuery}
          />
          <div className="filter-row" aria-label="Search format filter">
            {(["all", "markdown", "html"] as const).map((format) => (
              <button
                className={searchFormat === format ? "is-active" : ""}
                key={format}
                onClick={() => setSearchFormat(format)}
                type="button"
              >
                {format === "all" ? "All" : format.toUpperCase()}
              </button>
            ))}
          </div>
          {searchResults.length > 0 ? (
            <div className="search-results">
              {searchResults.map((result, index) => (
                <button
                  aria-label={`Open search result ${result.title}`}
                  className={`search-result ${index === selectedSearchIndex ? "is-active" : ""}`}
                  key={result.id}
                  onClick={() => openSearchResult(result)}
                  type="button"
                >
                  <strong>{result.title}</strong>
                  <small>
                    {result.format.toUpperCase()} / {new Date(result.updatedAt).toLocaleDateString()}
                  </small>
                  <span>{result.snippet.replace(/<\/?mark>/g, "")}</span>
                </button>
              ))}
            </div>
          ) : null}
        </section>
        <section className="sidebar-panel" aria-labelledby="recent-heading">
          <h2 id="recent-heading">Recent</h2>
          <div className="note-list" data-total={notes.length}>
            {visibleNotes.map((note) => (
              <button
                className={`note-row ${note.id === activeId ? "is-active" : ""}`}
                key={note.id}
                onClick={() => setActiveId(note.id)}
                type="button"
              >
                <span>
                  <strong>{note.title}</strong>
                  <small>{note.format.toUpperCase()}</small>
                </span>
                <em>{note.byteSize}B</em>
              </button>
            ))}
          </div>
        </section>
        <section className="sidebar-panel" aria-labelledby="transfer-heading">
          <h2 id="transfer-heading">Transfer</h2>
          <input
            aria-label="Import path"
            className="search-input"
            onChange={(event) => setImportPathValue(event.currentTarget.value)}
            placeholder="vault path"
            value={importPathValue}
          />
          <button className="wide-command" onClick={() => void importFromPath()} type="button">
            Import
          </button>
          <input
            aria-label="Export path"
            className="search-input"
            onChange={(event) => setExportPathValue(event.currentTarget.value)}
            placeholder="export path"
            value={exportPathValue}
          />
          <div className="filter-row" aria-label="Export mode">
            <button onClick={() => void exportActiveNote(false)} type="button">
              Source
            </button>
            <button onClick={() => void exportActiveNote(true)} type="button">
              Bundle
            </button>
            <button disabled type="button">
              {transferState}
            </button>
          </div>
        </section>
        <section className="sidebar-panel" aria-labelledby="reliability-heading">
          <h2 id="reliability-heading">Reliability</h2>
          <input
            aria-label="Backup path"
            className="search-input"
            onChange={(event) => setBackupPathValue(event.currentTarget.value)}
            placeholder="backup db path"
            value={backupPathValue}
          />
          <div className="filter-row" aria-label="Reliability actions">
            <button onClick={() => void runIntegrityCheck()} type="button">
              Check
            </button>
            <button onClick={() => void backupToPath()} type="button">
              Backup
            </button>
            <button onClick={() => void repairIndex()} type="button">
              Repair
            </button>
          </div>
          <p className="panel-note">{reliabilityState}</p>
        </section>
      </aside>

      <section className="content-inset" aria-label="Current note">
        <header className="top-strip">
          <span>PHASE 6</span>
          <span>No cloud sync</span>
          <span>{saveState}</span>
        </header>

        <section className="report-head">
          <p className="eyebrow">RELIABILITY</p>
          <div>
            <h1>{activeNote?.title ?? "Loading local notes."}</h1>
            <p>
              Notes stay local-first with markdown and sandboxed HTML previews. Search uses the
              content index. Artifact workflows, backups, integrity checks, and repair paths keep the vault dependable.
            </p>
          </div>
        </section>

        <div className="tab-row" role="tablist" aria-label="Note modes">
          {(["preview", "source", "split", "exports", "annotations", "compare"] as const).map(
            (mode) => (
              <button
                aria-selected={renderMode === mode}
                className={`tab ${renderMode === mode ? "is-active" : ""}`}
                key={mode}
                onClick={() => setRenderMode(mode)}
                role="tab"
                type="button"
              >
                {modeLabel(mode)}
              </button>
            ),
          )}
          <button className="tab" type="button" onClick={() => void copySource()}>
            Copy
          </button>
          <button className="tab" type="button" onClick={exportSource}>
            Export
          </button>
          <button className="tab" type="button" onClick={() => void renameActiveNote()}>
            Rename
          </button>
          <button className="tab" type="button" onClick={() => void deleteActiveNote()}>
            Delete
          </button>
        </div>

        <div className="work-grid">
          <section
            className={`document-surface render-mode-${renderMode}`}
            aria-labelledby="doc-heading"
          >
            <p className="eyebrow">{activeNote?.format.toUpperCase() ?? "NOTE"}</p>
            <h2 id="doc-heading">Source</h2>
            {renderMode !== "preview" && renderMode !== "exports" && renderMode !== "annotations" && renderMode !== "compare" ? (
              <textarea
                aria-label="Note content"
                className="editor"
                onChange={(event) => setDraft(event.currentTarget.value)}
                spellCheck={false}
                value={draft}
              />
            ) : null}
            {renderMode !== "source" && renderMode !== "exports" && renderMode !== "annotations" && renderMode !== "compare" && activeNote?.format === "markdown" ? (
              <article
                aria-label="Markdown preview"
                className="preview-surface markdown-preview"
                dangerouslySetInnerHTML={{ __html: renderedMarkdown }}
              />
            ) : null}
            {renderMode !== "source" && renderMode !== "exports" && renderMode !== "annotations" && renderMode !== "compare" && activeNote?.format === "html" ? (
              <iframe
                aria-label="HTML artifact preview"
                className="preview-surface html-preview"
                sandbox="allow-scripts"
                srcDoc={sandboxDocument}
                title="HTML artifact preview"
              />
            ) : null}
            {renderMode === "exports" ? (
              <div className="preview-surface export-panel" aria-label="Artifact exports">
                <button type="button" onClick={() => void copySource()}>
                  Copy Source
                </button>
                <button type="button" onClick={exportSource}>
                  Export Source
                </button>
                <button type="button" onClick={() => void navigator.clipboard?.writeText(draft)}>
                  Copy Prompt
                </button>
                <button type="button" onClick={() => void navigator.clipboard?.writeText(JSON.stringify({ title: activeNote?.title, content: draft }))}>
                  Copy JSON
                </button>
              </div>
            ) : null}
            {renderMode === "annotations" ? (
              <div className="preview-surface annotation-panel" aria-label="Artifact annotations">
                <textarea
                  aria-label="Annotation text"
                  onChange={(event) => setAnnotationDraft(event.currentTarget.value)}
                  placeholder="Review note"
                  value={annotationDraft}
                />
                <button type="button" onClick={saveAnnotation}>
                  Save Annotation
                </button>
                {annotations
                  .filter((annotation) => annotation.noteId === activeNote?.id)
                  .map((annotation, index) => (
                    <p key={`${annotation.noteId}-${index}`}>{annotation.text}</p>
                  ))}
              </div>
            ) : null}
            {renderMode === "compare" ? (
              <div className="preview-surface comparison-grid" aria-label="Artifact comparison">
                {notes
                  .filter((note) => note.format === "html")
                  .slice(0, 6)
                  .map((note) => (
                    <button key={note.id} onClick={() => setActiveId(note.id)} type="button">
                      <strong>{note.title}</strong>
                      <small>Tradeoff: fast static scan before opening full iframe.</small>
                    </button>
                  ))}
              </div>
            ) : null}
          </section>

          <aside className="right-rail" aria-label="Sources and local status">
            <section>
              <h2>Health</h2>
              {healthRows.map((item) => (
                <div className={`health-row is-${item.tone}`} key={item.label}>
                  <span>{item.label}</span>
                  <strong>{item.value}</strong>
                </div>
              ))}
            </section>
            <section>
              <h2>Selected</h2>
              <p>{activeNote ? `${activeNote.byteSize} bytes, ${activeNote.format}` : "No note selected"}</p>
              <p>Preview source commits after typing settles. HTML runs in an isolated script sandbox.</p>
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}

function categoryLabel(category: ArtifactCategory) {
  const labels: Record<ArtifactCategory, string> = {
    planning: "Planning",
    review: "Review",
    research: "Research",
    prototype: "Prototype",
  };

  return labels[category];
}

function modeLabel(mode: RenderMode) {
  const labels: Record<RenderMode, string> = {
    preview: "Preview",
    source: "Source",
    split: "Split",
    exports: "Exports",
    annotations: "Notes",
    compare: "Compare",
  };

  return labels[mode];
}

function artifactTemplate(category: ArtifactCategory) {
  const title = `${categoryLabel(category)} Artifact`;
  const payload = `Review ${categoryLabel(category).toLowerCase()} artifact`;

  return [
    "<!doctype html>",
    '<section data-o-note-artifact-version="1" data-category="' + category + '">',
    "  <style>",
    "    body{font-family:Inter,system-ui,sans-serif;margin:0}",
    "    .artifact{display:grid;gap:18px;max-width:900px}",
    "    .panel{border:1px solid #222;padding:16px}",
    "    button{border:1px solid #222;background:#fff;padding:8px 10px;font:12px ui-monospace,monospace;text-transform:uppercase}",
    "  </style>",
    '  <main class="artifact">',
    `    <h1>${title}</h1>`,
    '    <section class="panel"><h2>Context</h2><p>Summarize source context, tradeoffs, and decision pressure here.</p></section>',
    '    <section class="panel"><h2>Review</h2><p>Use annotations in o-note to capture decisions without editing the artifact source.</p></section>',
    `    <button onclick="parent.postMessage({version:1,noteId:'template',artifactId:'${category}',command:'copy_markdown',payload:'# ${payload}'}, '*')">Copy Prompt</button>`,
    "  </main>",
    "</section>",
  ].join("\n");
}
