/** Determina a data de pagamento para um lançamento da quinzena. */
export function pagamentoDaQuinzena(data: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(data);
  if (!match) throw new Error("Data inválida");
  const ano = Number(match[1]);
  const mes = Number(match[2]);
  const dia = Number(match[3]);
  const dataUtc = new Date(0);
  dataUtc.setUTCFullYear(ano, mes - 1, dia);
  dataUtc.setUTCHours(0, 0, 0, 0);
  if (ano < 1 || dataUtc.getUTCFullYear() !== ano || dataUtc.getUTCMonth() !== mes - 1 || dataUtc.getUTCDate() !== dia) {
    throw new Error("Data inválida");
  }
  if (dia <= 5) return `${match[1]}-${match[2]}-05`;
  if (dia <= 20) return `${match[1]}-${match[2]}-20`;
  dataUtc.setUTCMonth(dataUtc.getUTCMonth() + 1, 5);
  return dataUtc.toISOString().slice(0, 10);
}

/** Diária em centavos, arredondada ao centavo mais próximo. */
export function diariaEmCentavos(salario: bigint): bigint {
  if (salario < 0n) throw new Error("Salário inválido");
  return (salario + 15n) / 30n;
}

export type LancamentoQuinzena = {
  data: string;
  tipo: "FALTA_INJUSTIFICADA" | "FALTA_JUSTIFICADA" | "ADIANTAMENTO" | "PRODUCAO";
  valorCentavos?: bigint;
};

/** Prévia calculada com lançamentos fictícios; não representa fechamento ou pagamento. */
export function preverQuinzena(
  salario: bigint,
  pagamento: string,
  lancamentos: LancamentoQuinzena[],
) {
  if (salario < 0n) throw new Error("Salário inválido");
  if (pagamentoDaQuinzena(pagamento) !== pagamento) {
    throw new Error("Pagamento deve ocorrer no dia 05 ou 20");
  }
  let faltas = 0n;
  let adiantamentos = 0n;
  let producao = 0n;
  for (const lancamento of lancamentos) {
    if (pagamentoDaQuinzena(lancamento.data) !== pagamento) continue;
    if (lancamento.tipo === "FALTA_INJUSTIFICADA") {
      faltas += diariaEmCentavos(salario);
    } else if (lancamento.tipo === "ADIANTAMENTO" || lancamento.tipo === "PRODUCAO") {
      const valor = lancamento.valorCentavos;
      if (valor === undefined || valor <= 0n) throw new Error("Valor do lançamento inválido");
      if (lancamento.tipo === "ADIANTAMENTO") adiantamentos += valor;
      else producao += valor;
    }
  }
  const base = salario / 2n + (pagamento.endsWith("-05") ? salario % 2n : 0n);
  return { base, faltas, adiantamentos, producao, total: base + producao - faltas - adiantamentos };
}
