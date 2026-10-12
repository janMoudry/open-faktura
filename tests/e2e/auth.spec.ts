import { test, expect } from "@playwright/test";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

// Each test uses a disposable DB/server, never bootstraps the developer's account.
test("bootstrap, login, reload and logout against Rust and PostgreSQL", async ({
  page,
}, info) => {
  test.setTimeout(60000);
  const database = `auth_e2e_${randomBytes(8).toString("hex")}`;
  const directory = mkdtempSync(join(tmpdir(), "faktura-auth-"));
  const secret = randomBytes(32).toString("hex");
  writeFileSync(join(directory, "bootstrap"), secret, { mode: 0o600 });
  const port = 3200 + info.workerIndex;
  const sql = (query: string) =>
    execFileSync(
      "docker",
      [
        "compose",
        "-f",
        "infra/compose.yaml",
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        "faktura",
        "-d",
        "postgres",
        "-v",
        "ON_ERROR_STOP=1",
        "-c",
        query,
      ],
      { stdio: "pipe" },
    );
  let server: ChildProcess | undefined;
  sql(`CREATE DATABASE ${database}`);
  try {
    server = spawn("target/debug/faktura-server", [], {
      env: {
        ...process.env,
        APP_ENV: "development",
        DATABASE_URL: `postgres://faktura:faktura@127.0.0.1:54329/${database}`,
        DATABASE_URL_FILE: undefined,
        API_ADDRESS: `127.0.0.1:${port}`,
        BOOTSTRAP_SECRET_FILE: join(directory, "bootstrap"),
        CORS_ORIGINS: "http://127.0.0.1:5173",
      },
      stdio: "ignore",
    });
    await expect
      .poll(async () =>
        fetch(`http://127.0.0.1:${port}/api/ready`)
          .then((r) => r.status)
          .catch(() => 0),
      )
      .toBe(200);
    await page.route("**/api/**", async (route) => {
      const url = new URL(route.request().url());
      const response = await route.fetch({
        url: `http://127.0.0.1:${port}${url.pathname}`,
      });
      await route.fulfill({ response });
    });
    await page.goto("/");
    await page.getByLabel("E-mail", { exact: true }).fill("admin@example.test");
    await page
      .getByLabel("Heslo", { exact: true })
      .fill("Synthetic password 123!");
    await page.getByLabel("Zopakujte heslo").fill("Synthetic password 123!");
    await page.getByLabel("Instalační klíč", { exact: true }).fill(secret);
    await page.getByRole("button", { name: "Založit správce" }).click();
    await expect(page.getByRole("status")).toContainText("Správce byl založen");
    await page.getByLabel("Heslo", { exact: true }).fill("wrong password");
    await page.getByRole("button", { name: "Přihlásit se" }).click();
    await expect(page.getByRole("alert")).toContainText("nejsou platné");
    await page
      .getByLabel("Heslo", { exact: true })
      .fill("Synthetic password 123!");
    await page.getByRole("button", { name: "Přihlásit se" }).click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Fakturační profily",
    );
    await page.reload();
    await expect(
      page.getByText("Fakturujete sami na sebe?", { exact: false }),
    ).toBeVisible();
    expect(await page.evaluate(() => document.cookie)).not.toContain(
      "faktura_session",
    );
    await page.getByRole("button", { name: "Uživatelské menu" }).click();
    await page.getByRole("button", { name: "Odhlásit se" }).click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Přihlášení",
    );
    await page.goto("/#/setup");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Přihlášení",
    );
    await expect(
      page.getByLabel("Instalační klíč", { exact: true }),
    ).toHaveCount(0);
  } finally {
    if (server && server.exitCode === null) {
      const exited = new Promise((resolve) => server!.once("exit", resolve));
      server.kill("SIGINT");
      await exited;
    }
    sql(`DROP DATABASE ${database} WITH (FORCE)`);
    rmSync(directory, { recursive: true, force: true });
  }
});
