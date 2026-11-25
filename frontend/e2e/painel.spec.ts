import { test, expect } from "@playwright/test";
test("sessão, navegação, filtros e revogação", async ({ page }) => {
  const origin = process.env.E2E_ORIGIN || "http://127.0.0.1:8080";
  if (new URL(origin).hostname !== "127.0.0.1")
    throw Error("O teste exige loopback");
  await page.goto("/");
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(page.getByRole("alert").first()).toBeVisible();
  await expect(page.getByLabel("E-mail")).toHaveAttribute("aria-invalid", "true");
  await page
    .getByLabel("E-mail")
    .fill(process.env.E2E_EMAIL || "admin@local.test");
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD || "");
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Visão geral" }),
  ).toBeVisible();
  await expect(
    page.getByText("Cobranças pendentes", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Alternar tema" }).click();
  await expect(page.locator(".shell")).toHaveClass(/dark/);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(390);
  await page.getByRole("button", { name: "Ver extrato" }).click();
  await page.getByLabel("Buscar movimentação").fill("Horizonte");
  await expect(
    page.getByRole("cell", { name: "Projeto Horizonte" }),
  ).toBeVisible();
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await expect(page.getByRole("button", { name: "Fechar menu" })).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("button", { name: "Cobranças Pix" }).click();
  await expect(page.locator("#conteudo")).toBeFocused();
  const txid = page.getByLabel("Identificador da cobrança (txid)");
  const consult = page.getByRole("button", { name: "Consultar cobrança" });
  await txid.fill("abc");
  await consult.click();
  await expect(page.getByRole("alert")).toHaveText(
    "Informe de 26 a 35 letras ou números.",
  );
  await txid.fill("DEMO000000000000000000000001");
  await consult.click();
  await expect(
    page.getByRole("heading", { name: "Cobrança ativa" }),
  ).toBeVisible();
  await expect(page.getByText(/2\.120,00/)).toBeVisible();
  await txid.fill("DEMO000000000000000000000009");
  await consult.click();
  await expect(page.getByRole("alert")).toHaveText(
    "Cobrança Pix não encontrada na conta.",
  );
  await expect(
    page.getByRole("heading", { name: "Cobrança ativa" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("button", { name: "Contas MOCK" }).click();
  const form = page.locator("form").filter({ has: page.getByLabel("Fornecedor ou cliente") }).first();
  await form.getByLabel("Fornecedor ou cliente").fill("Fornecedor de teste");
  await form.getByLabel("Descrição").fill("Teste de falha");
  await form.getByLabel("Valor (R$)").fill("12,50");
  await form.getByLabel("Vencimento").fill("2026-12-01");
  await page.route("**/api/mock/financeiro", async (route) => {
    if (route.request().method() === "POST") await route.fulfill({ status: 503 });
    else await route.continue();
  });
  await form.getByRole("button", { name: "Salvar lançamento fictício" }).click();
  await expect(page.getByRole("alert")).toContainText("Não foi possível salvar");
  await expect(form.getByLabel("Fornecedor ou cliente")).toHaveValue("Fornecedor de teste");
  await expect(form.getByLabel("Valor (R$)")).toHaveValue("12,50");
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("button", { name: "Auditoria" }).click();
  await expect(page.getByText("Sessão iniciada").first()).toBeVisible();
  await page.getByRole("button", { name: "Sair" }).click();
  await expect(
    page.getByRole("heading", { name: "Acessar painel" }),
  ).toBeVisible();
  await expect((await page.request.get("/api/dashboard")).status()).toBe(401);
});
