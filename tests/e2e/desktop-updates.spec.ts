import { test, expect } from "@playwright/test";
import { mockRedesignWorkspace } from "./redesign-fixture";
import cs from "../../apps/web/src/locales/cs.json";
import en from "../../apps/web/src/locales/en.json";

// Browser-only IPC simulation: this test never downloads or installs native software.
for (const language of ["cs", "en"] as const) {
  for (const enabled of [false, true]) {
    test(`desktop update confirmation ${language} enabled=${enabled}`, async ({
      page,
    }, info) => {
      const t = language === "cs" ? cs : en;
      await mockRedesignWorkspace(page, { language });
      await page.addInitScript(
        ({ enabled }) => {
          const calls: { command: string; args: unknown }[] = [];
          Object.assign(window, {
            isTauri: true,
            updateCalls: calls,
            failInstall: false,
            __TAURI_INTERNALS__: {
              invoke: async (command: string, args: unknown) => {
                calls.push({ command, args });
                if (command === "desktop_updater_status")
                  return { enabled, current_version: "0.1.0" };
                if (command === "desktop_updater_check")
                  return {
                    version: "0.2.0",
                    current_version: "0.1.0",
                    notes: "Synthetic release <script>untrusted</script>",
                    token: "synthetic-confirmation-token",
                  };
                if (command === "desktop_updater_install") {
                  if (
                    (window as unknown as { failInstall: boolean }).failInstall
                  )
                    throw new Error("updater_signature_failed");
                  return;
                }
                throw new Error("Unexpected mock IPC command");
              },
            },
          });
        },
        { enabled },
      );
      await page.goto("/#/help");
      await expect(
        page.getByRole("heading", { name: t.desktopUpdateTitle }),
      ).toBeVisible();
      const calls = () =>
        page.evaluate(
          () =>
            (
              window as unknown as {
                updateCalls: { command: string; args: unknown }[];
              }
            ).updateCalls,
        );
      await expect.poll(async () => (await calls()).length).toBe(1);
      if (!enabled) {
        await expect(page.getByText(t.desktopUpdateDisabled)).toBeVisible();
        await expect(
          page.getByRole("button", { name: t.desktopUpdateCheck }),
        ).toHaveCount(0);
        return;
      }
      await page
        .getByRole("button", { name: t.desktopUpdateCheck, exact: true })
        .click();
      await expect(
        page.getByText("Synthetic release <script>untrusted</script>", {
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: t.desktopUpdateInstall, exact: true })
        .click();
      const dialog = page.getByRole("dialog", {
        name: t.desktopUpdateInstall,
        exact: true,
      });
      const install = dialog.getByRole("button", {
        name: t.desktopUpdateConfirm,
        exact: true,
      });
      await expect(install).toBeDisabled();
      await page.keyboard.press("Escape");
      expect(
        (await calls()).some((c) => c.command === "desktop_updater_install"),
      ).toBe(false);
      await page
        .getByRole("button", { name: t.desktopUpdateInstall, exact: true })
        .click();
      await dialog
        .getByRole("checkbox", { name: t.desktopUpdateSaved })
        .check();
      await install.click();
      await expect(
        page.getByText(t.desktopUpdateInstalled, { exact: true }),
      ).toBeVisible();
      expect(
        (await calls()).filter((c) => c.command === "desktop_updater_install"),
      ).toEqual([
        {
          command: "desktop_updater_install",
          args: {
            version: "0.2.0",
            token: "synthetic-confirmation-token",
            confirmed: true,
          },
        },
      ]);
      await page.evaluate(() => {
        (window as unknown as { failInstall: boolean }).failInstall = true;
      });
      await page
        .getByRole("button", { name: t.desktopUpdateCheck, exact: true })
        .click();
      await page
        .getByRole("button", { name: t.desktopUpdateInstall, exact: true })
        .click();
      await expect(dialog.getByRole("checkbox")).not.toBeChecked();
      await dialog.getByRole("checkbox").check();
      await install.click();
      await expect(page.getByRole("alert")).toContainText(
        t.desktopUpdateFailed,
      );
      await expect(
        page.getByRole("button", { name: t.desktopUpdateInstall, exact: true }),
      ).toHaveCount(0);
      await page.screenshot({
        path: info.outputPath("desktop-update-error.png"),
        fullPage: true,
      });
    });
  }
}
