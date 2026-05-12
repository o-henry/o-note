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

export type IndexHealth = {
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

export type RenderMode = "preview" | "source" | "split";
