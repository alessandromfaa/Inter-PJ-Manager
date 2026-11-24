import { expect, test } from "@playwright/test";
import jsQR from "jsqr";

test("emissão MOCK, reenvio após perda da resposta e leitura do QR", async ({
  page,
}) => {
  const origin = process.env.E2E_ORIGIN || "http://127.0.0.1:8080";
  if (new URL(origin).hostname !== "127.0.0.1")
    throw new Error("O teste exige loopback");
  await page.goto("/");
  await page
    .getByLabel("E-mail")
    .fill(process.env.E2E_EMAIL || "admin@local.test");
  await page.getByLabel("Senha").fill(process.env.E2E_PASSWORD || "");
  await page.getByRole("button", { name: "Entrar", exact: true }).click();
  const openIssue = async () => {
    await page.getByRole("button", { name: "Cobranças Pix" }).click();
    await page.getByRole("button", { name: "Nova cobrança em MOCK" }).click();
  };
  await openIssue();
  await page.getByLabel("Valor da cobrança (R$)").fill("42,50");

  let originalTxid = "";
  await page.route(
    "**/api/mock/pix/cobrancas",
    async (route) => {
      const issued = await route.fetch();
      expect(issued.status()).toBe(201);
      originalTxid = (await issued.json()).txid;
      await route.abort("failed");
    },
    { times: 1 },
  );
  await page.getByRole("button", { name: "Emitir cobrança fictícia" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "A confirmação não chegou.",
  );
  await page.reload();
  await openIssue();
  const replay = page.waitForResponse((r) =>
    r.url().endsWith("/api/mock/pix/cobrancas"),
  );
  await page.getByRole("button", { name: "Retomar emissão" }).click();
  expect((await replay).status()).toBe(200);
  await expect(page.getByText(originalTxid, { exact: true })).toBeVisible();
  const readCode = page.getByLabel("Pix Copia e Cola (MOCK)");
  const text = await readCode.inputValue();
  const bitmap = await page
    .getByRole("img", { name: "QR Code da cobrança fictícia" })
    .evaluate((el) => {
      const canvas = el as HTMLCanvasElement;
      const { data } = canvas
        .getContext("2d")!
        .getImageData(0, 0, canvas.width, canvas.height);
      const chunks = [];
      for (let i = 0; i < data.length; i += 8192)
        chunks.push(String.fromCharCode(...data.subarray(i, i + 8192)));
      return {
        width: canvas.width,
        height: canvas.height,
        pixels: btoa(chunks.join("")),
      };
    });
  const decoded = jsQR(
    new Uint8ClampedArray(Buffer.from(bitmap.pixels, "base64")),
    bitmap.width,
    bitmap.height,
  );
  expect(decoded?.data).toBe(text);
  expect(text).toContain("pix.invalid/");
  await page.getByRole("button", { name: "Voltar à consulta" }).click();
  await page.getByLabel("Identificador da cobrança (txid)").fill(originalTxid);
  await page.getByRole("button", { name: "Consultar cobrança" }).click();
  await expect(readCode).toHaveValue(text);
});
