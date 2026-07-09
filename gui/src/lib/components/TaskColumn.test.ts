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

  it("windows a long list to one page and reveals more on demand", async () => {
    const many = Array.from({ length: 60 }, (_, i) => ({
      id: `#${i + 1}`,
      subject: `Task ${i + 1}`,
      status: "open",
    }));
    render(TaskColumn, { props: { tasks: many } });
    // First page (50) is rendered; the 51st row is withheld.
    expect(screen.getByText("Task 50")).toBeInTheDocument();
    expect(screen.queryByText("Task 51")).not.toBeInTheDocument();
    // The manual fallback reveals the remaining resident rows.
    screen.getByText("Load more").click();
    await tick();
    expect(screen.getByText("Task 60")).toBeInTheDocument();
  });

  it("calls onLoadMore when the resident page is exhausted", async () => {
    let called = 0;
    render(TaskColumn, {
      props: { tasks, hasMore: true, onLoadMore: () => (called += 1) },
    });
    // Only two resident tasks (< page), so the button fetches the next page.
    screen.getByText("Load more").click();
    await tick();
    expect(called).toBe(1);
  });
});
