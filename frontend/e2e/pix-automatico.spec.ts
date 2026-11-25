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
  const authorizationCreated = page.waitForResponse((response) =>
    response.url().endsWith("/api/mock/pix-automatico/recorrencias") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Criar solicitação fictícia" }).click();
  const authorizationResponse = await authorizationCreated;
  expect(authorizationResponse.status()).toBe(201);
  const authorization = await authorizationResponse.json();
  await expect(page.getByText(payer)).toBeVisible();

  await page.getByRole("button", { name: "Simular aceite do pagador" }).click();
  await expect(page.getByText(`${payer} · ATIVA`)).toBeVisible();
  await page.getByLabel("Autorização ativa").selectOption({ label: `${payer} · MENSAL` });
  await page.getByLabel("Competência").fill(start);
  await page.getByLabel("Valor da cobrança (R$)").fill("23,45");
  const chargeCreated = page.waitForResponse((response) =>
    response.url().endsWith("/api/mock/pix-automatico/cobrancas") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Criar cobrança fictícia" }).click();
  const chargeResponse = await chargeCreated;
  expect(chargeResponse.status()).toBe(201);
  const charge = await chargeResponse.json();
  const duplicate = await page.request.post(`${origin}/api/mock/pix-automatico/cobrancas`, {
    data: chargeResponse.request().postDataJSON(),
    headers: { Origin: origin, "X-PJ-Request": "1" },
  });
  expect(duplicate.status()).toBe(200);
  expect((await duplicate.json()).id).toBe(charge.id);
  await expect(page.getByText(`${start} · R$ 23,45 · CRIADA`)).toBeVisible();
  const settled = page.waitForResponse((response) =>
    response.url().endsWith("/api/mock/pix-automatico/cobrancas/liquidar") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Simular liquidação" }).click();
  const settlement = await settled;
  const settlementResult = await settlement.json();
  const repeatedSettlement = await page.request.post(`${origin}/api/mock/pix-automatico/cobrancas/liquidar`, {
    data: { id: charge.id },
    headers: { Origin: origin, "X-PJ-Request": "1" },
  });
  expect(repeatedSettlement.ok()).toBe(true);
  expect((await repeatedSettlement.json()).end_to_end_id).toBe(settlementResult.end_to_end_id);
  const settledCharge = page.locator("article.event").filter({ hasText: settlementResult.end_to_end_id });
  await expect(settledCharge).toContainText(`${start} · R$ 23,45 · LIQUIDADA`);
  await expect(settledCharge).toContainText(`Identificador de demonstração: ${settlementResult.end_to_end_id}`);
  const authorizationCard = page.locator("article.event").filter({ hasText: payer });
  await authorizationCard.getByRole("button", { name: "Cancelar autorização" }).click();
  await expect(authorizationCard).toContainText(`${payer} · CANCELADA`);
  const repeatedCancellation = await page.request.post(`${origin}/api/mock/pix-automatico/recorrencias/cancelar`, {
    data: { id: authorization.id },
    headers: { Origin: origin, "X-PJ-Request": "1" },
  });
  expect(repeatedCancellation.ok()).toBe(true);
});
