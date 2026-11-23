export type Identity = { email: string; company: string; roles: string[] };
export type Entry = {
  date: string;
  description: string;
  category: string;
  cents: number;
};
export type Snapshot = {
  source: "MOCK";
  as_of: string;
  opening: number;
  available: number;
  incoming: number;
  outgoing: number;
  entries: Entry[];
  summary: {
    settled_charge_cents: number;
    pending_charge_cents: number;
    scheduled_payment_cents: number;
    unreconciled_count: number;
  };
};
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
export async function api<T>(path: string, data?: unknown): Promise<T> {
  const response = await fetch(
    "/api/" + path,
    data === undefined
      ? { cache: "no-store" }
      : {
          method: "POST",
          headers: { "Content-Type": "application/json", "X-PJ-Request": "1" },
          body: JSON.stringify(data),
        },
  );
  if (!response.ok) {
    const msg = await response.json().catch(() => null);
    throw new ApiError(
      response.status,
      typeof msg === "string" ? msg : "Falha ao carregar. Tente novamente.",
    );
  }
  return response.json();
}
const currency = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
});
export function brl(cents: number) {
  if (!Number.isSafeInteger(cents))
    throw new RangeError("Valor em centavos fora da faixa suportada.");
  const magnitude = BigInt(Math.abs(cents));
  const fraction = (magnitude % 100n).toString().padStart(2, "0");
  const parts = currency.formatToParts(magnitude / 100n);
  return (
    (cents < 0 ? "-" : "") +
    parts.map((p) => (p.type === "fraction" ? fraction : p.value)).join("")
  );
}
export const date = (iso: string) =>
  new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "short",
    timeZone: "UTC",
  }).format(new Date(iso + "T12:00:00Z"));
