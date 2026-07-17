// WebdriverIO i18n test: switching the interface language in Settings actually
// re-renders the UI in that language (the translation reaches the real DOM, not
// just the key map). Language-agnostic: asserts Cyrillic appears for Russian and
// the English label returns for English, without hardcoding a translation.
const hasCyrillic = (s) => /[А-Яа-яЁё]/.test(s);

describe("interface language", () => {
  it("switches the rendered language from the settings", async () => {
    // Open Settings via the header icon (client-side route).
    await $(".settings-link").click();
    const title = await $("h1");
    await title.waitForExist();

    // Switch to Russian: the heading must render Cyrillic text.
    await $('input[name="language"][value="ru"]').click();
    await browser.waitUntil(async () => hasCyrillic(await title.getText()), {
      timeout: 5000,
      timeoutMsg: "settings heading did not switch to Russian",
    });

    // Switch back to English: the heading returns to the English label.
    await $('input[name="language"][value="en"]').click();
    await browser.waitUntil(
      async () => (await title.getText()) === "Settings",
      {
        timeout: 5000,
        timeoutMsg: "settings heading did not switch back to English",
      },
    );
  });
});
