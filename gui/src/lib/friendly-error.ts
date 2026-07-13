import type { Key } from "./locales/en";

// Turn a raw backend error string into user-facing text. The core `Error`
// Display prefixes messages with a technical variant tag (`api:`/`auth:`/`io:`/
// `config:`) that means nothing to a user, so strip it. The list commands emit
// the stable `not-signed-in` sentinel for an OpenProject server with no stored
// token, which maps to a friendly message with a link to Settings.
export interface FriendlyError {
  text: string;
  // The "not signed in" case: the caller should offer a link to Settings.
  notSignedIn: boolean;
}

const KIND_PREFIXES = ["api", "auth", "io", "config", "internal"];

export function friendlyError(
  raw: string,
  t: (key: Key) => string,
): FriendlyError {
  const s = raw.trim();
  if (s === "not-signed-in") {
    return { text: t("error.notSignedIn"), notSignedIn: true };
  }
  // Strip a leading "kind: " tag, repeatedly (Display can nest, e.g. "api: ...").
  let msg = s;
  for (;;) {
    const m = /^([a-z]+):\s+([\s\S]*)$/.exec(msg);
    if (m && KIND_PREFIXES.includes(m[1])) {
      msg = m[2];
    } else {
      break;
    }
  }
  return { text: msg, notSignedIn: false };
}
