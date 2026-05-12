import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App shell", () => {
  it("renders the Phase 0 local-first shell", () => {
    render(<App />);

    expect(screen.getByText("o-note")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Report" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("Fast local notes with sandboxed HTML artifacts.")).toBeInTheDocument();
    expect(screen.getByText("Sandboxed")).toBeInTheDocument();
  });
});
