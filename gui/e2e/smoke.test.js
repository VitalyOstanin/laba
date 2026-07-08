// WebdriverIO smoke test: the app starts and both dashboard columns render.
describe("dashboard smoke", () => {
  it("shows both columns", async () => {
    const tasks = await $("section[aria-label='My tasks']");
    const notifs = await $("section[aria-label='Notifications']");
    await expect(tasks).toBeExisting();
    await expect(notifs).toBeExisting();
  });
});
