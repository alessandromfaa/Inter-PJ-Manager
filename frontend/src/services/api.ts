export type Identity = {
  id: string;
  company_id: string;
  email: string;
  company: string;
  roles: string[];
};
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
  if (response.status === 204) return undefined as T;
  return response.json();
}
const currency = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
});
export function brl(cents: number | string) {
  if (typeof cents === "number" && !Number.isSafeInteger(cents))
    throw new RangeError("Valor em centavos fora da faixa suportada.");
  const value = BigInt(cents);
  const magnitude = value < 0n ? -value : value;
  const fraction = (magnitude % 100n).toString().padStart(2, "0");
  const parts = currency.formatToParts(magnitude / 100n);
  return (
    (value < 0n ? "-" : "") +
    parts.map((p) => (p.type === "fraction" ? fraction : p.value)).join("")
  );
}
export const date = (iso: string) =>
  new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "short",
    timeZone: "UTC",
  }).format(new Date(iso + "T12:00:00Z"));

export type Mode = "MOCK" | "SANDBOX" | "PRODUCTION";
export function environmentLabel(mode: Mode) {
  return {
    MOCK: "MOCK · Demonstração",
    SANDBOX: "SANDBOX · Homologação",
    PRODUCTION: "PRODUCTION · Leitura",
  }[mode];
}
export type Reading = {
  synchronized_at: string;
  imported_from: string;
  imported_until: string;
  balance: {
    available: string;
    credit_limit: string | null;
    blocked_check: string | null;
    blocked_admin: string | null;
    blocked_court: string | null;
    reference_date: string | null;
  };
};
export type LiveSnapshot = {
  pix_lookup_enabled: boolean;
  source: "SANDBOX" | "PRODUCTION";
  reading: Reading | null;
  certificate_expires_at: string;
  last_error: string | null;
};
