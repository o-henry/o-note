import type { IndexHealth, ShellNote } from "../shared/domain";

const notes: ShellNote[] = [
  {
    id: "n-001",
    title: "HTML artifact workflow",
    format: "html",
    updatedAt: "10:42",
    status: "indexed",
  },
  {
    id: "n-002",
    title: "Obsidian import notes",
    format: "markdown",
    updatedAt: "09:18",
    status: "pending",
  },
  {
    id: "n-003",
    title: "Phase 0 foundation",
    format: "markdown",
    updatedAt: "Yesterday",
    status: "draft",
  },
];

const health: IndexHealth[] = [
  { label: "Shell", value: "Ready", tone: "steady" },
  { label: "Index", value: "Idle", tone: "steady" },
  { label: "HTML", value: "Sandboxed", tone: "active" },
  { label: "Storage", value: "Local", tone: "steady" },
];

export function App() {
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
        <section className="sidebar-panel" aria-labelledby="recent-heading">
          <h2 id="recent-heading">Recent</h2>
          <div className="note-list">
            {notes.map((note) => (
              <button className="note-row" key={note.id} type="button">
                <span>
                  <strong>{note.title}</strong>
                  <small>{note.format.toUpperCase()}</small>
                </span>
                <em>{note.updatedAt}</em>
              </button>
            ))}
          </div>
        </section>
      </aside>

      <section className="content-inset" aria-label="Current note">
        <header className="top-strip">
          <span>PHASE 0</span>
          <span>No cloud sync</span>
          <span>SQLite first</span>
        </header>

        <section className="report-head">
          <p className="eyebrow">PERFORMANCE CONTRACT</p>
          <div>
            <h1>Fast local notes with sandboxed HTML artifacts.</h1>
            <p>
              The foundation shell keeps metadata, rendering, and indexing separate so large
              vaults do not turn every interaction into a filesystem scan.
            </p>
          </div>
        </section>

        <div className="tab-row" role="tablist" aria-label="Note modes">
          <button className="tab is-active" type="button" role="tab" aria-selected="true">
            Report
          </button>
          <button className="tab" type="button" role="tab" aria-selected="false">
            Source
          </button>
          <button className="tab" type="button" role="tab" aria-selected="false">
            Index
          </button>
        </div>

        <div className="work-grid">
          <section className="document-surface" aria-labelledby="doc-heading">
            <p className="eyebrow">CURRENT ARTIFACT</p>
            <h2 id="doc-heading">Phase 0 shell baseline</h2>
            <p>
              This is the first app surface: a Bullpen-inspired desktop shell with a sidebar,
              rectangular tabs, local-only status, and a right rail for sources and health.
            </p>
            <div className="metric-grid">
              <div>
                <span>Cold shell</span>
                <strong>&lt;= 1.5s</strong>
              </div>
              <div>
                <span>Search warm</span>
                <strong>&lt;= 100ms</strong>
              </div>
              <div>
                <span>HTML mount</span>
                <strong>&lt;= 250ms</strong>
              </div>
            </div>
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
              <h2>Sources</h2>
              <p>Docs, generated artifacts, imports, and backlinks will live here.</p>
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}
