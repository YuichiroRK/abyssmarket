import type {
  ApiErrorBody,
  Creature,
  CreatureClass,
  Expedition,
  ExpeditionDetails,
  InventoryItem,
  SessionResponse,
} from "./types";

const API_ROOT = "/api";
const SESSION_KEY = "abyssmarket.session";

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export function setAccessToken(token: string | null): void {
  if (token) {
    sessionStorage.setItem(SESSION_KEY, token);
  } else {
    sessionStorage.removeItem(SESSION_KEY);
  }
}

export function hasSession(): boolean {
  return sessionStorage.getItem(SESSION_KEY) !== null;
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  const token = sessionStorage.getItem(SESSION_KEY);
  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (init.body) headers.set("Content-Type", "application/json");

  const response = await fetch(`${API_ROOT}${path}`, { ...init, headers });
  if (response.status === 204) return undefined as T;

  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    payload = null;
  }

  if (!response.ok) {
    const message = isApiErrorBody(payload) ? payload.error : "No se pudo completar la solicitud.";
    throw new ApiError(message, response.status);
  }
  return payload as T;
}

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  return typeof value === "object" && value !== null && "error" in value && typeof value.error === "string";
}

function post<T>(path: string, body: unknown): Promise<T> {
  return request<T>(path, { method: "POST", body: JSON.stringify(body) });
}

export const api = {
  register(username: string, password: string): Promise<SessionResponse> {
    return post("/auth/register", { username, password });
  },

  login(username: string, password: string): Promise<SessionResponse> {
    return post("/auth/login", { username, password });
  },

  logout(): Promise<void> {
    return request<void>("/auth/session", { method: "DELETE" });
  },

  getCreature(): Promise<Creature> {
    return request<Creature>("/creatures/me");
  },

  createCreature(name: string, creatureClass: CreatureClass): Promise<Creature> {
    return post("/creatures", { name, class: creatureClass });
  },

  getInventory(): Promise<InventoryItem[]> {
    return request<InventoryItem[]>("/inventory");
  },

  startExpedition(depth: number): Promise<Expedition> {
    return post("/expeditions", { depth });
  },

  getLatestExpedition(): Promise<ExpeditionDetails | null> {
    return request<ExpeditionDetails | null>("/expeditions/latest");
  },
};
