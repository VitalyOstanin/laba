import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/svelte";
import { tick } from "svelte";
import TaskColumn from "./TaskColumn.svelte";
import { filterText } from "../store";
import { makeTask } from "../test-fixtures";
import type { ServerInfo } from "../types";

// A minimal GitHub server so the scope tabs (GitHub-only) render. Capabilities
// are stubbed because TaskColumn reads capabilities.status_filters/task_detail.
const caps = { status_filters: false, task_detail: "none" };
const ghServer = {
  name: "gh",
  backend: "github",
  capabilities: caps,
} as unknown as ServerInfo;

afterEach(() => {
  cleanup();
  filterText.set("");
});

describe("TaskColumn", () => {
  const tasks = [
    makeTask({
      id: { display: "#1", raw: "1" },
      title: "Fix pagination",
      status: "open",
    }),
    makeTask({
      id: { display: "#2", raw: "2" },
      title: "Cache avatars",
      status: "closed",
    }),
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
    const many = Array.from({ length: 60 }, (_, i) =>
      makeTask({
        id: { display: `#${i + 1}`, raw: String(i + 1) },
        title: `Task ${i + 1}`,
        status: "open",
      }),
    );
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

  describe("scope tabs (My repos / Others)", () => {
    const mixed = [
      makeTask({
        id: { display: "me/app#1", raw: "1" },
        title: "Own repo task",
        status: "open",
        project: "me/app",
        mine: true,
      }),
      makeTask({
        id: { display: "acme/tool#2", raw: "2" },
        title: "Followed repo task",
        status: "open",
        project: "acme/tool",
        mine: false,
      }),
    ];

    it("defaults to My repos and hides tasks in other repos", () => {
      render(TaskColumn, { props: { tasks: mixed, server: ghServer } });
      expect(screen.getByText("Own repo task")).toBeInTheDocument();
      expect(screen.queryByText("Followed repo task")).not.toBeInTheDocument();
      // The two scopes are disjoint: 1 mine, 1 other.
      const scopeNav = screen.getByLabelText("Task scope");
      expect(scopeNav).toHaveTextContent("My repos");
      expect(scopeNav).toHaveTextContent("Others");
    });

    it("switches to Others to show only repos the user does not own", async () => {
      render(TaskColumn, { props: { tasks: mixed, server: ghServer } });
      screen.getByText("Others").click();
      await tick();
      expect(screen.getByText("Followed repo task")).toBeInTheDocument();
      expect(screen.queryByText("Own repo task")).not.toBeInTheDocument();
    });

    it("hides the scope tabs for a non-GitHub server", () => {
      const opServer = {
        name: "op",
        backend: "openproject",
        capabilities: caps,
      } as unknown as ServerInfo;
      render(TaskColumn, { props: { tasks: mixed, server: opServer } });
      expect(screen.queryByLabelText("Task scope")).not.toBeInTheDocument();
      // Without scope, both tasks show.
      expect(screen.getByText("Own repo task")).toBeInTheDocument();
      expect(screen.getByText("Followed repo task")).toBeInTheDocument();
    });
  });
});
