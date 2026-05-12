import { invoke } from "@tauri-apps/api/core";
import type {
  CreateNoteInput,
  IndexHealth,
  NoteDetail,
  NoteSummary,
  SearchNotesQuery,
  SearchResult,
  UpdateNoteInput,
} from "../shared/domain";

type ListNotesQuery = {
  limit?: number;
  offset?: number;
};

type NotesApi = {
  createNote(input: CreateNoteInput): Promise<NoteDetail>;
  listNotes(query?: ListNotesQuery): Promise<NoteSummary[]>;
  getNote(id: string): Promise<NoteDetail | null>;
  updateNote(input: UpdateNoteInput): Promise<NoteDetail>;
  renameNote(id: string, title: string): Promise<NoteSummary>;
  deleteNote(id: string): Promise<void>;
  searchNotes(query: SearchNotesQuery): Promise<SearchResult[]>;
  indexHealth(): Promise<IndexHealth>;
};

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const notesApi: NotesApi = window.__TAURI_INTERNALS__ ? tauriNotesApi() : memoryNotesApi();

function tauriNotesApi(): NotesApi {
  return {
    createNote: (input) => invoke("create_note", { input }),
    listNotes: (query = {}) => invoke("list_notes", { query }),
    getNote: (id) => invoke("get_note", { id }),
    updateNote: (input) => invoke("update_note", { input }),
    renameNote: (id, title) => invoke("rename_note", { id, title }),
    deleteNote: (id) => invoke("delete_note", { id }),
    searchNotes: (query) => invoke("search_notes", { query }),
    indexHealth: () => invoke("index_health"),
  };
}

function memoryNotesApi(): NotesApi {
  let notes: NoteDetail[] = [
    makeNote("n-001", "HTML artifact workflow", "html", htmlSeed()),
    makeNote("n-002", "Obsidian import notes", "markdown", markdownSeed()),
    makeNote("n-003", "Phase 1 core notes", "markdown", "# Phase 1\n\nCRUD is local-first."),
  ];

  return {
    async createNote(input) {
      const note = makeNote(`n-${crypto.randomUUID()}`, input.title, input.format, input.content);
      notes = [note, ...notes];
      return note;
    },
    async listNotes(query = {}) {
      const limit = query.limit ?? 100;
      const offset = query.offset ?? 0;
      return notes.slice(offset, offset + limit).map(toSummary);
    },
    async getNote(id) {
      return notes.find((note) => note.id === id) ?? null;
    },
    async updateNote(input) {
      const now = new Date().toISOString();
      const note = mustFind(notes, input.id);
      const updated = {
        ...note,
        content: input.content,
        byteSize: new TextEncoder().encode(input.content).length,
        contentHash: String(input.content.length),
        updatedAt: now,
      };
      notes = notes.map((item) => (item.id === input.id ? updated : item));
      return updated;
    },
    async renameNote(id, title) {
      const note = mustFind(notes, id);
      const updated = { ...note, title: title.trim() || "Untitled", updatedAt: new Date().toISOString() };
      notes = notes.map((item) => (item.id === id ? updated : item));
      return toSummary(updated);
    },
    async deleteNote(id) {
      notes = notes.filter((note) => note.id !== id);
    },
    async searchNotes(query) {
      const normalized = query.query.trim().toLowerCase();
      if (!normalized) return [];

      return notes
        .filter((note) => {
          const haystack = `${note.title} ${note.content}`.toLowerCase();
          const formatMatches = !query.format || note.format === query.format;
          const tagMatches = !query.tag || note.content.includes(`#${query.tag}`);
          return formatMatches && tagMatches && haystack.includes(normalized);
        })
        .slice(0, query.limit ?? 20)
        .map((note) => ({
          id: note.id,
          title: note.title,
          format: note.format,
          updatedAt: note.updatedAt,
          snippet: createSnippet(note.content, normalized),
        }));
    },
    async indexHealth() {
      return { pending: 0, indexed: notes.length, failed: 0 };
    },
  };
}

function makeNote(
  id: string,
  title: string,
  format: NoteDetail["format"],
  content: string,
): NoteDetail {
  const now = new Date().toISOString();

  return {
    id,
    title,
    format,
    content,
    contentHash: String(content.length),
    byteSize: new TextEncoder().encode(content).length,
    createdAt: now,
    updatedAt: now,
  };
}

function toSummary(note: NoteDetail): NoteSummary {
  return {
    id: note.id,
    title: note.title,
    format: note.format,
    updatedAt: note.updatedAt,
    byteSize: note.byteSize,
  };
}

function mustFind(notes: NoteDetail[], id: string): NoteDetail {
  const note = notes.find((item) => item.id === id);

  if (!note) {
    throw new Error(`Missing note ${id}`);
  }

  return note;
}

function markdownSeed() {
  return [
    "# Obsidian import notes",
    "",
    "- Markdown stays first-class.",
    "- Bodies load only after a note is selected.",
  ].join("\n");
}

function htmlSeed() {
  return [
    "<section>",
    "  <h1>HTML Artifact Workflow</h1>",
    "  <p>Sandboxed previews arrive in Phase 2.</p>",
    "</section>",
  ].join("\n");
}

function createSnippet(content: string, query: string) {
  const index = content.toLowerCase().indexOf(query);
  if (index === -1) return content.slice(0, 80);
  const start = Math.max(0, index - 24);
  const end = Math.min(content.length, index + query.length + 48);
  return content.slice(start, end);
}
