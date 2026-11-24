import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  const origin = process.env.E2E_ORIGIN!;
  expect(origin).toBe("http://127.0.0.1:18080");
  const environment = await page.request.get("/api/ambiente", {
    maxRedirects: 0,
  });
  expect(environment.ok()).toBe(true);
  expect(await environment.json()).toBe("MOCK");

  await page.goto("/");
  await page
    .getByLabel("E-mail")
    .fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(
    page.getByText("MOCK · Demonstração", { exact: true }),
  ).toBeVisible();
});

test("cadastra cliente, emite cobrança V3 e concilia recebimento por boleto", async ({
  page,
}) => {
  const name = "Cliente de teste " + randomUUID();
  const due = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000)
    .toISOString()
    .slice(0, 10);

  await page.getByRole("button", { name: "Recebimentos", exact: true }).click();
  await page.getByLabel("Nome do cliente").fill(name);
  await page.getByRole("button", { name: "Cadastrar cliente" }).click();
  await expect(page.getByRole("status")).toHaveText("Cliente cadastrado.");

  const customer = page.getByRole("row").filter({ hasText: name });
  const next = page.getByRole("button", { name: "Próxima", exact: true });
  let customerPage = 1;
  await expect(page.getByText("Página 1", { exact: true })).toBeVisible();
  while ((await customer.count()) === 0 && (await next.isEnabled())) {
    await next.click();
    customerPage++;
    await expect(
      page.getByText(`Página ${customerPage}`, { exact: true }),
    ).toBeVisible();
  }
  await customer.getByRole("button", { name: "Preparar V3" }).click();
  await page.getByLabel("Valor nominal (R$)").fill("38,41");
  await page.getByLabel("Vencimento").fill(due);
  const submitted = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/mock/cobrancas-v3") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Solicitar emissão em MOCK" }).click();
  const accepted = await submitted;
  expect(accepted.status()).toBe(202);
  const { codigo_solicitacao, situacao } = await accepted.json();
  expect(situacao).toBe("EM_PROCESSAMENTO");
  await expect(page.getByRole("status")).toContainText(codigo_solicitacao);

  const charge = page.getByRole("row").filter({ hasText: codigo_solicitacao });
  await expect(charge).toContainText("A RECEBER", { timeout: 12000 });
  await expect(charge).toContainText(due.split("-").reverse().join("/"));
  await charge
    .getByRole("button", { name: "Simular recebimento por BOLETO" })
    .click();
  await expect(page.getByRole("status")).toContainText(
    "Solicitação registrada.",
  );
  await expect(charge).toContainText("RECEBIDO");

  await page.getByRole("button", { name: "Conciliações", exact: true }).click();
  await expect(charge).toContainText("CONCILIADO", { timeout: 12000 });
  await expect(charge).toContainText("BOLETO");
  await expect(charge).toContainText("38,41");
});

test("simula o recebimento Pix e concilia pelo identificador da cobrança", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Cobranças Pix", exact: true })
    .click();
  await page.getByRole("button", { name: "Nova cobrança em MOCK" }).click();
  await page.getByLabel("Valor da cobrança (R$)").fill("42,51");
  const issued = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/mock/pix/cobrancas") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Emitir cobrança fictícia" }).click();
  const creation = await issued;
  expect(creation.status()).toBe(201);
  const { txid } = await creation.json();
  await expect(page.getByText(txid, { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Recebimentos", exact: true }).click();
  await page.getByRole("button", { name: "Cobranças Pix emitidas" }).click();
  const charge = page.getByRole("row").filter({ hasText: txid });
  const received = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/mock/recebimentos/pix") &&
      response.request().method() === "POST",
  );
  await charge.getByRole("button", { name: "Simular recebimento Pix" }).click();
  const simulation = await received;
  expect(simulation.status()).toBe(202);
  const { end_to_end_id } = await simulation.json();
  expect(end_to_end_id).toMatch(/^E[A-Za-z0-9]{31}$/);
  await expect(charge).toContainText("CONCLUIDA", { timeout: 12000 });

  await page.getByRole("button", { name: "Conciliações", exact: true }).click();
  const receipt = page.getByRole("row").filter({ hasText: end_to_end_id });
  await expect(receipt).toContainText("CONCILIADO", { timeout: 12000 });
  await expect(receipt).toContainText(txid);
  await expect(receipt).toContainText("42,51");
});

test("rejeita sucesso vazio quando a operação exige representação", async ({
  page,
}) => {
  await page.route("**/api/mock/pix/cobrancas", async (route) => {
    await route.fulfill({ status: 201, body: "" });
  });
  await page
    .getByRole("button", { name: "Cobranças Pix", exact: true })
    .click();
  await page.getByRole("button", { name: "Nova cobrança em MOCK" }).click();
  await page.getByLabel("Valor da cobrança (R$)").fill("10,00");
  await page.getByRole("button", { name: "Emitir cobrança fictícia" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "A confirmação não chegou.",
  );
});

test("rejeita 204 vazio quando a operação exige representação", async ({
  page,
}) => {
  await page.route("**/api/mock/pix/cobrancas", async (route) => {
    await route.fulfill({ status: 204, body: "" });
  });
  await page.getByRole("button", { name: "Cobranças Pix", exact: true }).click();
  await page.getByRole("button", { name: "Nova cobrança em MOCK" }).click();
  await page.getByLabel("Valor da cobrança (R$)").fill("10,00");
  await page.getByRole("button", { name: "Emitir cobrança fictícia" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "A confirmação não chegou.",
  );
});
