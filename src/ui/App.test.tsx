import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

describe("App core notes", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the Phase 6 local-first shell", async () => {
    render(<App />);

    expect(screen.getByText("o-note")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(await screen.findByText("HTML artifact workflow")).toBeInTheDocument();
    expect(screen.getByText(/repair paths keep the vault dependable/)).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Split" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("Sandboxed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Planning" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Import path" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Export path" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Backup path" })).toBeInTheDocument();
  });

  it("creates, edits, renames, and deletes a Markdown note", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "prompt").mockReturnValue("Renamed from test");
    render(<App />);

    await user.click(screen.getByRole("button", { name: "New MD" }));
    const editor = await screen.findByRole("textbox", { name: "Note content" });
    expect(await screen.findAllByText("Untitled note")).toHaveLength(2);

    await user.clear(editor);
    await user.type(editor, "# Test note");
    expect(await screen.findByText("Queued")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Saved")).toBeInTheDocument(), {
      timeout: 1500,
    });

    await user.click(screen.getByRole("button", { name: "Rename" }));
    expect(await screen.findAllByText("Renamed from test")).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => {
      expect(screen.queryByText("Renamed from test")).not.toBeInTheDocument();
    });
  }, 10_000);

  it("searches note content and opens the matching note", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.type(screen.getByRole("textbox", { name: "Search notes" }), "Obsidian");
    await user.click(
      await screen.findByRole("button", { name: "Open search result Obsidian import notes" }),
    );

    expect(await screen.findByDisplayValue(/# Obsidian import notes/)).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Search notes" })).toHaveValue("");
  });

  it("creates a templated artifact and saves an annotation", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Planning" }));
    await waitFor(() => expect(screen.getAllByText("Planning artifact").length).toBeGreaterThan(0));
    await user.click(screen.getByRole("tab", { name: "Notes" }));
    await user.type(screen.getByRole("textbox", { name: "Annotation text" }), "Looks ready");
    await user.click(screen.getByRole("button", { name: "Save Annotation" }));

    expect(screen.getByText("Looks ready")).toBeInTheDocument();
  });
});
