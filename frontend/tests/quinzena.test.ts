import { describe, expect, it } from "vitest";
import { diariaEmCentavos, pagamentoDaQuinzena, preverQuinzena, salarioNaData } from "../src/services/quinzena";

it("aplica o salário vigente na data do pagamento sem alterar períodos anteriores", () => {
  const mudancas = [{ vigente_a_partir: "2099-02-05", salario_centavos: "400000" }, { vigente_a_partir: "2099-01-20", salario_centavos: "350000" }];
  expect(salarioNaData("300000", "2099-01-05", mudancas)).toBe(300000n);
  expect(salarioNaData("300000", "2099-01-20", mudancas)).toBe(350000n);
  expect(salarioNaData("300000", "2099-02-20", mudancas)).toBe(400000n);
});

describe("pagamento da quinzena", () => {
  it("aplica os limites definidos pelo proprietário", () => {
    expect(pagamentoDaQuinzena("2026-11-06")).toBe("2026-11-20");
    expect(pagamentoDaQuinzena("2026-11-20")).toBe("2026-11-20");
    expect(pagamentoDaQuinzena("2026-11-21")).toBe("2026-12-05");
    expect(pagamentoDaQuinzena("2026-12-04")).toBe("2026-12-05");
    expect(pagamentoDaQuinzena("2026-12-05")).toBe("2026-12-05");
    expect(pagamentoDaQuinzena("2026-12-31")).toBe("2027-01-05");
    expect(pagamentoDaQuinzena("2028-02-29")).toBe("2028-03-05");
  });

  it("recusa datas inválidas", () => {
    expect(() => pagamentoDaQuinzena("2026-02-29")).toThrow();
    expect(() => pagamentoDaQuinzena("2026-13-01")).toThrow();
  });
});

describe("prévia da quinzena", () => {
  it("soma produção e desconta faltas e adiantamentos no período correto", () => {
    expect(preverQuinzena(300000n, "2026-12-05", [
      { data: "2026-11-21", tipo: "FALTA_INJUSTIFICADA" },
      { data: "2026-12-05", tipo: "ADIANTAMENTO", valorCentavos: 20000n },
      { data: "2026-12-04", tipo: "PRODUCAO", valorCentavos: 15000n },
      { data: "2026-12-01", tipo: "FALTA_JUSTIFICADA" },
      { data: "2026-11-20", tipo: "ADIANTAMENTO", valorCentavos: 90000n },
    ])).toEqual({ base: 150000n, faltas: 10000n, adiantamentos: 20000n, producao: 15000n, total: 135000n });
  });

  it("atribui ao dia 05 o centavo restante e rejeita valores inválidos", () => {
    expect(preverQuinzena(100001n, "2026-12-05", []).base).toBe(50001n);
    expect(preverQuinzena(100001n, "2026-12-20", []).base).toBe(50000n);
    expect(() => preverQuinzena(100000n, "2026-12-05", [{ data: "2026-12-02", tipo: "PRODUCAO", valorCentavos: 0n }])).toThrow();
  });
});

describe("diária", () => {
  it("divide o salário por 30 com arredondamento em centavos", () => {
    expect(diariaEmCentavos(150000n)).toBe(5000n);
    expect(diariaEmCentavos(100000n)).toBe(3333n);
    expect(diariaEmCentavos(100005n)).toBe(3334n);
    expect(() => diariaEmCentavos(-1n)).toThrow();
  });
});
