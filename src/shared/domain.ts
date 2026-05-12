export type NoteFormat = "markdown" | "html";

export type ShellNote = {
  id: string;
  title: string;
  format: NoteFormat;
  updatedAt: string;
  status: "indexed" | "pending" | "draft";
};

export type IndexHealth = {
  label: string;
  value: string;
  tone: "steady" | "active" | "warn";
};
