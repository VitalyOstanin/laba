import { describe, it, expect } from "vitest";
import { friendlyError } from "./friendly-error";
import type { Key } from "./locales/en";

// A translator stub: returns a marker for the not-signed-in key, echoes others.
const t = (key: Key): string =>
  key === "error.notSignedIn" ? "NOT_SIGNED_IN" : key;

describe("friendlyError", () => {
  it("maps the not-signed-in sentinel to a friendly message with a flag", () => {
    const r = friendlyError("not-signed-in", t);
    expect(r).toEqual({ text: "NOT_SIGNED_IN", notSignedIn: true });
  });

  it("strips a single technical kind prefix", () => {
    expect(friendlyError("api: 401 Unauthorized", t).text).toBe(
      "401 Unauthorized",
    );
    expect(friendlyError("auth: no token for 'x'", t).text).toBe(
      "no token for 'x'",
    );
  });

  it("strips nested kind prefixes", () => {
    expect(friendlyError("api: io: connection refused", t).text).toBe(
      "connection refused",
    );
  });

  it("leaves an unprefixed message unchanged", () => {
    expect(friendlyError("something went wrong", t).text).toBe(
      "something went wrong",
    );
  });

  it("does not strip a non-kind word before a colon", () => {
    expect(friendlyError("server 'demo': gone", t).text).toBe(
      "server 'demo': gone",
    );
  });

  it("never flags a non-sentinel error as not-signed-in", () => {
    expect(friendlyError("api: 401", t).notSignedIn).toBe(false);
  });
});
