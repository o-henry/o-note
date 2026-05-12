import { useCallback, useEffect, useMemo, useState } from "react";
import { createSandboxDocument, isAllowedArtifactMessage, renderMarkdown } from "../rendering/render";
import { notesApi } from "../services/notes";
import type { IndexHealth, NoteDetail, NoteFormat, NoteSummary, RenderMode } from "../shared/domain";

const health: IndexHealth[] = [
  { label: "Shell", value: "Ready", tone: "steady" },
  { label: "Index", value: "Idle", tone: "steady" },
  { label: "HTML", value: "Sandboxed", tone: "active" },
  { label: "Storage", value: "Local", tone: "steady" },
];

export function App() {
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [activeNote, setActiveNote] = useState<NoteDetail | null>(null);
  const [draft, setDraft] = useState("");
  const [previewSource, setPreviewSource] = useState("");
  const [renderMode, setRenderMode] = useState<RenderMode>("split");
  const [saveState, setSaveState] = useState("Idle");
  const visibleNotes = useMemo(() => notes.slice(0, 100), [notes]);
  const renderedMarkdown = useMemo(() => {
    if (activeNote?.format !== "markdown") return "";
    return renderMarkdown(previewSource);
  }, [activeNote?.format, previewSource]);
  const sandboxDocument = useMemo(() => {
    if (activeNote?.format !== "html") return "";
    return createSandboxDocument(previewSource);
  }, [activeNote?.format, previewSource]);

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
    const handle = (event: MessageEvent) => {
      if (!event.data || typeof event.data !== "object") return;
      setSaveState(isAllowedArtifactMessage(event.data) ? "Bridge allowed" : "Bridge blocked");
    };

    window.addEventListener("message", handle);

    return () => window.removeEventListener("message", handle);
  }, []);

  async function createNote(format: NoteFormat) {
    const created = await notesApi.createNote({
      title: format === "html" ? "Untitled HTML artifact" : "Untitled note",
      format,
      content: format === "html" ? "<section>\n  <h1>Untitled</h1>\n</section>" : "# Untitled\n",
    });
    await refreshNotes(created.id);
    setActiveId(created.id);
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

  function exportSource() {
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
  }

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
      </aside>

      <section className="content-inset" aria-label="Current note">
        <header className="top-strip">
          <span>PHASE 1</span>
          <span>No cloud sync</span>
          <span>{saveState}</span>
        </header>

        <section className="report-head">
          <p className="eyebrow">CORE NOTES</p>
          <div>
            <h1>{activeNote?.title ?? "Loading local notes."}</h1>
            <p>
              Metadata lists stay separate from note bodies. The editor loads only the selected
              note and autosaves in the background after typing settles.
            </p>
          </div>
        </section>

        <div className="tab-row" role="tablist" aria-label="Note modes">
          <button
            className={`tab ${renderMode === "preview" ? "is-active" : ""}`}
            type="button"
            role="tab"
            aria-selected={renderMode === "preview"}
            onClick={() => setRenderMode("preview")}
          >
            Preview
          </button>
          <button
            className={`tab ${renderMode === "source" ? "is-active" : ""}`}
            type="button"
            role="tab"
            aria-selected={renderMode === "source"}
            onClick={() => setRenderMode("source")}
          >
            Source
          </button>
          <button
            className={`tab ${renderMode === "split" ? "is-active" : ""}`}
            type="button"
            role="tab"
            aria-selected={renderMode === "split"}
            onClick={() => setRenderMode("split")}
          >
            Split
          </button>
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
            {renderMode !== "preview" ? (
              <textarea
                aria-label="Note content"
                className="editor"
                onChange={(event) => setDraft(event.currentTarget.value)}
                spellCheck={false}
                value={draft}
              />
            ) : null}
            {renderMode !== "source" && activeNote?.format === "markdown" ? (
              <article
                aria-label="Markdown preview"
                className="preview-surface markdown-preview"
                dangerouslySetInnerHTML={{ __html: renderedMarkdown }}
              />
            ) : null}
            {renderMode !== "source" && activeNote?.format === "html" ? (
              <iframe
                aria-label="HTML artifact preview"
                className="preview-surface html-preview"
                sandbox=""
                srcDoc={sandboxDocument}
                title="HTML artifact preview"
              />
            ) : null}
          </section>

          <aside className="right-rail" aria-label="Sources and local status">
            <section>
              <h2>Health</h2>
              {health.map((item) => (
                <div className={`health-row is-${item.tone}`} key={item.label}>
                  <span>{item.label}</span>
                  <strong>{item.value}</strong>
                </div>
              ))}
            </section>
            <section>
              <h2>Selected</h2>
              <p>{activeNote ? `${activeNote.byteSize} bytes, ${activeNote.format}` : "No note selected"}</p>
              <p>Preview source commits after typing settles. HTML runs in a static sandbox.</p>
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}
