import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";
import TaskColumn from "./TaskColumn.svelte";
import { filterText } from "../store";

afterEach(() => {
  cleanup();
  filterText.set("");
});

describe("TaskColumn", () => {
  const tasks = [
    { id: "#1", subject: "Fix pagination", status: "open" },
    { id: "#2", subject: "Cache avatars", status: "closed" },
  ];

  it("shows all tasks with empty filter", () => {
    render(TaskColumn, { props: { tasks } });
    expect(screen.getByText("Fix pagination")).toBeInTheDocument();
    expect(screen.getByText("Cache avatars")).toBeInTheDocument();
  });

  it("filters reactively", async () => {
    render(TaskColumn, { props: { tasks } });
    filterText.set("closed");
    await tick();
    expect(screen.queryByText("Fix pagination")).not.toBeInTheDocument();
    expect(screen.getByText("Cache avatars")).toBeInTheDocument();
  });
});
