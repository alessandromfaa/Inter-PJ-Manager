import { describe, it, expect } from "vitest";
import { brl, date } from "../src/services/api";
describe("Apresentação brasileira", () => {
  it("preserva centavos e sinal", () =>
    expect(brl(-1234).replace(/\s/g, " ")).toBe("-R$ 12,34"));
  it("preserva dia de competência", () =>
    expect(date("2026-11-04")).toMatch(/^04/));
});
