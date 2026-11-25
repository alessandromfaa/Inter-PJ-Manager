import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function verificar(page: Page) {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(results.violations.map(({ id, nodes }) => ({
    id,
    elementos: nodes.map((node) => node.target),
  }))).toEqual([]);
}

test("acessibilidade das principais telas em desktop e celular", async ({ page }) => {
  await page.goto("/");
  await verificar(page);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(page.getByRole("alert").first()).toBeVisible();
  await verificar(page);
  await page.getByLabel("E-mail").fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
  for (const name of ["Visão geral", "Extrato", "Cobranças Pix", "Recebimentos", "Pagamentos Pix", "Contas MOCK", "Pix Automático", "Auditoria"]) {
    await page.getByRole("button", { name, exact: true }).click();
    await expect(page.getByRole("heading", { name, exact: true }).first()).toBeVisible();
    await verificar(page);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await verificar(page);
  await page.getByRole("button", { name: "Contas MOCK", exact: true }).click();
  await page.getByText("Gerenciar categorias e centros de custo").click();
  await page.getByText("Recorrências locais", { exact: true }).click();
  await verificar(page);
  await page.getByRole("button", { name: "Alternar tema" }).click();
  await verificar(page);
});
