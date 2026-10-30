export interface Player {
  id: number;
  username: string;
}

export interface SessionResponse {
  player: Player;
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
}

export type CreatureClass = "explorer" | "tank" | "hacker";

export interface Creature {
  id: number;
  name: string;
  class: CreatureClass;
}

export interface InventoryItem {
  slug: string;
  name: string;
  quantity: number;
}

export interface Expedition {
  id: number;
  depth: number;
  seed: number;
  status: "running" | "completed";
  ends_at: string;
}

export interface ExpeditionEvent {
  tick: number;
  description: string;
  loot: Array<{ item: string; quantity: number }>;
  created_at: string;
}

export interface ExpeditionDetails {
  expedition: Expedition;
  events: ExpeditionEvent[];
}

export interface ApiErrorBody {
  error: string;
}
