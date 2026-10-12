import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
for (const root of ["apps/web", "packages/ui"]) {
  const read = async (language) =>
    JSON.parse(await readFile(`${root}/src/locales/${language}.json`, "utf8"));
  const cs = await read("cs"),
    en = await read("en");
  assert.deepEqual(
    Object.keys(cs).sort(),
    Object.keys(en).sort(),
    `${root}: translation keys differ`,
  );
  const variables = (text) =>
    [...text.matchAll(/{{\s*(\w+)\s*}}/g)].map((m) => m[1]).sort();
  for (const key of Object.keys(cs)) {
    assert.equal(typeof cs[key], "string");
    assert.equal(typeof en[key], "string");
    assert.ok(
      cs[key].trim() && en[key].trim(),
      `${root}: empty translation ${key}`,
    );
    for (const value of [cs[key], en[key]])
      assert.ok(
        !/(?<!{){\w+}(?!})/.test(value),
        `${root}: single-brace interpolation in ${key}`,
      );
    assert.deepEqual(
      variables(cs[key]),
      variables(en[key]),
      `${root}: interpolation differs for ${key}`,
    );
  }
}
console.log("Translation keys and interpolation parameters match (cs/en).");
