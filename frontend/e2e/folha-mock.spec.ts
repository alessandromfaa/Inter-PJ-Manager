import { expect, test } from "@playwright/test";

test("simula quinzena sem gravar funcionários ou enviar pagamentos", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("E-mail").fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await page.getByRole("button", { name: "Folha MOCK" }).click();
  await page.getByLabel("Data do pagamento").fill("2026-12-05");
  await page.getByLabel("Data", { exact: true }).fill("2026-12-05");
  await page.getByRole("button", { name: "Adicionar à simulação" }).click();
  await expect(page.getByText("R$ 100,00")).toBeVisible();
  await expect(page.getByText("R$ 1.400,00")).toBeVisible();
  await page.getByRole("button", { name: "Remover" }).click();
  await expect(page.getByText("R$ 1.500,00")).toHaveCount(2);
});

test("persiste funcionário e lançamento fictícios na empresa", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("E-mail").fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await page.getByRole("button", { name: "Folha MOCK" }).click();
  const nome = `Funcionário de teste ${Date.now()}`;
  await page.getByLabel("Nome fictício").fill(nome);
  await page.getByLabel("Salário mensal fictício (R$)").fill("3000,00");
  await page.getByRole("button", { name: "Cadastrar funcionário fictício" }).click();
  await expect(page.getByRole("combobox", { name: "Funcionário", exact: true })).toContainText(nome);
  await page.getByLabel("Data do pagamento").fill("2026-12-05");
  await page.getByLabel("Data", { exact: true }).fill("2026-12-05");
  await page.getByRole("button", { name: "Salvar lançamento fictício" }).click();
  await expect(page.getByText("R$ 1.400,00")).toBeVisible();
  await page.reload();
  await page.getByRole("button", { name: "Folha MOCK" }).click();
  await page.getByRole("combobox", { name: "Funcionário", exact: true }).selectOption({ label: nome });
  await page.getByLabel("Data do pagamento").fill("2026-12-05");
  await expect(page.getByText("R$ 1.400,00")).toBeVisible();
});

test("repetição idempotente não duplica funcionário e rejeita dados diferentes", async ({ page, browser }) => {
  const anonimo = await browser.newContext();
  expect((await anonimo.request.get("http://127.0.0.1:18080/api/mock/folha/funcionarios")).status()).toBe(401);
  await anonimo.close();
  await page.goto("/");
  await page.getByLabel("E-mail").fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Visão geral" })).toBeVisible();
  const headers = { Origin: "http://127.0.0.1:18080", "X-PJ-Request": "1" };
  const dados = { nome: "Pessoa sintética idempotente", salario_centavos: "300000", idempotencia: crypto.randomUUID() };
  const primeira = await page.request.post("/api/mock/folha/funcionarios", { headers, data: dados });
  expect(primeira.status()).toBe(201);
  const segunda = await page.request.post("/api/mock/folha/funcionarios", { headers, data: dados });
  expect(segunda.status()).toBe(201);
  expect((await segunda.json()).id).toBe((await primeira.json()).id);
  const divergente = await page.request.post("/api/mock/folha/funcionarios", { headers, data: { ...dados, salario_centavos: "400000" } });
  expect(divergente.status()).toBe(409);
  const funcionarioId = (await primeira.json()).id;
  const lancamento = { funcionario_id: funcionarioId, tipo: "ADIANTAMENTO", data: "2026-12-05", valor_centavos: "1000", idempotencia: crypto.randomUUID() };
  const lancamento1 = await page.request.post("/api/mock/folha/lancamentos", { headers, data: lancamento });
  const lancamento2 = await page.request.post("/api/mock/folha/lancamentos", { headers, data: lancamento });
  expect(lancamento1.status()).toBe(201);
  expect((await lancamento2.json()).id).toBe((await lancamento1.json()).id);
  const outro = await page.request.post("/api/mock/folha/lancamentos", { headers, data: { ...lancamento, funcionario_id: crypto.randomUUID(), idempotencia: crypto.randomUUID() } });
  expect(outro.status()).toBe(404);
});
