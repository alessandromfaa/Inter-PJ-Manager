import { expect, test } from "@playwright/test";
test("leitura em homologação com respostas locais", async ({ page }) => {
  let synced = false;
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const reply = (json: unknown) => route.fulfill({ json });
    if (path === "/api/ambiente") return reply("SANDBOX");
    if (path === "/api/session")
      return reply({
        email: "admin@local.test",
        roles: ["ADMIN"],
        company: "Empresa de ensaio",
      });
    if (path === "/api/sincronizar") {
      expect(route.request().method()).toBe("POST");
      synced = true;
      return route.fulfill({ status: 204 });
    }
    if (path === "/api/dashboard")
      return reply({
        source: "SANDBOX",
        certificate_expires_at: "2030-01-01T00:00:00Z",
        last_error: null,
        reading: synced
          ? {
              balance: {
                available: "12025",
                blocked_admin: null,
                blocked_court: null,
                blocked_check: null,
                credit_limit: null,
                reference_date: null,
              },
              synchronized_at: "2026-11-23T00:00:00Z",
              imported_from: "2026-11-01",
              imported_until: "2026-11-23",
            }
          : null,
      });
    if (path === "/api/extrato")
      return reply({
        entries: [{ date: "2026-11-23", cents: "-532", kind: "TARIFA" }],
        has_more: false,
      });
    return route.abort();
  });
  await page.goto("/");
  await expect(page.getByText("SANDBOX · Homologação")).toBeVisible();
  await expect(
    page.getByText("Aguardando a primeira sincronização."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Sincronizar leitura" }).click();
  await expect(page.getByText(/120,25/)).toBeVisible();
  await page.getByRole("button", { name: "Extrato", exact: true }).click();
  await expect(page.getByRole("cell", { name: "TARIFA" })).toBeVisible();
});
