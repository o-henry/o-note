import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

describe("App core notes", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the Phase 1 local-first shell", async () => {
    render(<App />);

    expect(screen.getByText("o-note")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(await screen.findByText("HTML artifact workflow")).toBeInTheDocument();
    expect(screen.getByText(/Metadata lists stay separate from note bodies/)).toBeInTheDocument();
    expect(screen.getByText("Sandboxed")).toBeInTheDocument();
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
  });
});
