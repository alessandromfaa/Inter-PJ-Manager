import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";

test("Pix Automático MOCK percorre consentimento, cobrança e liquidação fictícios", async ({ page }) => {
  const origin = process.env.E2E_ORIGIN!;
  expect(origin).toBe("http://127.0.0.1:18080");
  const environment = await page.request.get("/api/ambiente", { maxRedirects: 0 });
  expect(environment.ok()).toBe(true);
  expect(await environment.json()).toBe("MOCK");

  await page.goto("/");
  await page.getByLabel("E-mail").fill(process.env.E2E_EMAIL!);
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD!);
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  await page.getByRole("button", { name: "Pix Automático" }).click();

  const marker = randomUUID();
  const payer = `Pagador sintético ${marker}`;
  const start = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
  await page.getByLabel("Nome fictício do pagador").fill(payer);
  await page.getByLabel("Descrição do vínculo").fill("Assinatura de teste");
  await page.getByLabel("Referência de contrato").fill(marker.slice(0, 30));
  await page.getByLabel("Início").fill(start);
  await page.getByLabel("Valor recorrente (R$)").fill("23,45");
  await page.getByRole("button", { name: "Criar solicitação fictícia" }).click();
  await expect(page.getByText(payer)).toBeVisible();

  await page.getByRole("button", { name: "Simular aceite do pagador" }).click();
  await expect(page.getByText(`${payer} · ATIVA`)).toBeVisible();
  await page.getByLabel("Autorização ativa").selectOption({ label: `${payer} · MENSAL` });
  await page.getByLabel("Competência").fill(start);
  await page.getByLabel("Valor da cobrança (R$)").fill("23,45");
  await page.getByRole("button", { name: "Criar cobrança fictícia" }).click();
  await expect(page.getByText(`${start} · R$ 23,45 · CRIADA`)).toBeVisible();
  await page.getByRole("button", { name: "Simular liquidação" }).click();
  await expect(page.getByText(`${start} · R$ 23,45 · LIQUIDADA`)).toBeVisible();
  await expect(page.getByText("Identificador de demonstração:")).toBeVisible();
});
