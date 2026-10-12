import { readdir, readFile } from "node:fs/promises";
async function walk(path) {
  const entries = await readdir(path, { withFileTypes: true });
  return (
    await Promise.all(
      entries.map((e) =>
        e.isDirectory() ? walk(`${path}/${e.name}`) : `${path}/${e.name}`,
      ),
    )
  ).flat();
}
const errors = [];
for (const root of ["apps/web/src", "packages/ui/src"])
  for (const file of await walk(root)) {
    const text = await readFile(file, "utf8");
    if (/#[0-9a-f]{3,8}\b|(?:bg|text|border|p|m|gap)-\[/.test(text))
      errors.push(`${file}: použijte design token`);
    if (/<h[1-6](?:\s|>)/.test(text))
      errors.push(`${file}: použijte sdílený Heading`);
    if (
      root.startsWith("packages") &&
      /from ['"].*(?:apps\/|api-client)/.test(text)
    )
      errors.push(`${file}: UI závisí na aplikaci/API`);
  }
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log("Token and UI boundary checks passed (limited static checks).");
