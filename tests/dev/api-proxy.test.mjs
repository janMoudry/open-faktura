import { test } from "node:test";
import assert from "node:assert/strict";
import { developmentApiOrigin } from "../../apps/web/dev/api-proxy.mjs";

test("isolated preview translates only its own explicit loopback origin", () => {
  for (const host of ["localhost:5174", "127.0.0.1:5174"])
    assert.equal(
      developmentApiOrigin(`http://${host}`, host),
      "http://127.0.0.1:5173",
    );
});

test("foreign, missing and mismatched origins remain subject to API rejection", () => {
  for (const origin of [
    undefined,
    "null",
    "https://untrusted.example",
    "http://127.0.0.1:5174.untrusted.example",
    "http://localhost:5174",
    "https://127.0.0.1:5174",
  ])
    assert.equal(developmentApiOrigin(origin, "127.0.0.1:5174"), origin);
  for (const host of [undefined, "untrusted.example:5174", "127.0.0.1:5173"])
    assert.equal(
      developmentApiOrigin("http://127.0.0.1:5174", host),
      "http://127.0.0.1:5174",
    );
});
