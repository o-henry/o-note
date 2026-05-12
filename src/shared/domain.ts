export type NoteFormat = "markdown" | "html";

export type NoteSummary = {
  id: string;
  title: string;
  format: NoteFormat;
  updatedAt: string;
  byteSize: number;
};

export type NoteDetail = NoteSummary & {
  content: string;
  contentHash: string;
  createdAt: string;
};

export type SearchResult = {
  id: string;
  title: string;
  format: NoteFormat;
  snippet: string;
  updatedAt: string;
};

export type IndexHealth = {
  pending: number;
  indexed: number;
  failed: number;
};

export type HealthRow = {
  label: string;
  value: string;
  tone: "steady" | "active" | "warn";
};

export type CreateNoteInput = {
  title: string;
  format: NoteFormat;
  content: string;
};

export type UpdateNoteInput = {
  id: string;
  content: string;
};

export type SearchNotesQuery = {
  query: string;
  limit?: number;
  format?: NoteFormat;
  tag?: string;
};

export type ImportPathInput = {
  path: string;
};

export type ImportReport = {
  runId: string;
  importedNotes: number;
  importedAttachments: number;
  skippedFiles: number;
};

export type ExportNoteInput = {
  id: string;
  path: string;
  bundle: boolean;
};

export type ExportReport = {
  outputPath: string;
  filesWritten: number;
};

export type BackupInput = {
  path: string;
};

export type ReliabilityReport = {
  status: string;
  detail: string;
};

export type RenderMode = "preview" | "source" | "split" | "exports" | "annotations" | "compare";
