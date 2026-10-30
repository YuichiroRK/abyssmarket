import { api, ApiError, hasSession, setAccessToken } from "./api/client";
import type { Creature, CreatureClass, ExpeditionDetails, InventoryItem } from "./api/types";
import type { AbyssScene } from "./scenes/AbyssScene";
import "./styles.css";

function element<T extends HTMLElement>(selector: string): T {
  const result = document.querySelector<T>(selector);
  if (!result) throw new Error(`Falta el elemento ${selector}`);
  return result;
}

const authPanel = element<HTMLElement>("#auth-panel");
const gamePanel = element<HTMLElement>("#game-panel");
const authForm = element<HTMLFormElement>("#auth-form");
const authTitle = element<HTMLElement>("#auth-panel h1");
const authSubmit = element<HTMLButtonElement>("#auth-submit");
const authModeButton = element<HTMLButtonElement>("#auth-mode");
const authError = element<HTMLElement>("#auth-error");
const creatureForm = element<HTMLFormElement>("#creature-form");
const creatureSummary = element<HTMLElement>("#creature-summary");
const creatureError = element<HTMLElement>("#creature-error");
const expeditionForm = element<HTMLFormElement>("#expedition-form");
const expeditionSubmit = element<HTMLButtonElement>("#expedition-submit");
const expeditionStatus = element<HTMLElement>("#expedition-status");
const expeditionError = element<HTMLElement>("#expedition-error");
const inventoryList = element<HTMLUListElement>("#inventory-list");
const logoutButton = element<HTMLButtonElement>("#logout");

let isRegistering = false;
let currentCreature: Creature | null = null;
let currentExpedition: ExpeditionDetails | null = null;
let pollingTimer: number | undefined;
let refreshInProgress = false;
let abyssScene: AbyssScene | null = null;
let abyssGame: import("phaser").Game | null = null;

async function startAbyssGame(): Promise<void> {
  if (abyssGame) return;
  const [{ default: Phaser }, { AbyssScene }] = await Promise.all([
    import("phaser"),
    import("./scenes/AbyssScene"),
  ]);
  if (abyssGame) return;
  abyssGame = new Phaser.Game({
    type: Phaser.CANVAS,
    width: 800,
    height: 480,
    parent: "abyss-canvas",
    backgroundColor: "#100d1d",
    pixelArt: true,
    render: { antialias: false, roundPixels: true },
    scale: { mode: Phaser.Scale.FIT, autoCenter: Phaser.Scale.CENTER_BOTH },
    scene: [AbyssScene],
  });
  abyssGame.events.once("ready", () => {
    const game = abyssGame;
    if (!game) return;
    abyssScene = game.scene.getScene("AbyssScene") as AbyssScene;
    updateAbyss();
  });
}

function updateAbyss(): void {
  abyssScene?.setView({
    creatureName: currentCreature?.name ?? null,
    creatureClass: currentCreature?.class ?? null,
    depth: currentExpedition?.expedition.depth ?? null,
    status: currentExpedition?.expedition.status ?? "idle",
  });
}

function showAuth(): void {
  authPanel.hidden = false;
  gamePanel.hidden = true;
  if (pollingTimer !== undefined) window.clearInterval(pollingTimer);
}

function setAuthMode(registering: boolean): void {
  isRegistering = registering;
  authTitle.innerHTML = registering
    ? 'Crea tu puesto <span>nocturno</span>'
    : 'Mercado <span>del Abismo</span>';
  authSubmit.textContent = registering ? "Crear cuenta" : "Entrar";
  authModeButton.textContent = registering
    ? "Ya tengo una cuenta"
    : "¿Primera vez? Crear una cuenta";
  element<HTMLInputElement>("#password").autocomplete = registering ? "new-password" : "current-password";
  authError.textContent = "";
}

async function authenticate(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  authSubmit.disabled = true;
  authError.textContent = "";
  const form = new FormData(authForm);
  const username = String(form.get("username") ?? "");
  const password = String(form.get("password") ?? "");

  try {
    const session = isRegistering
      ? await api.register(username, password)
      : await api.login(username, password);
    setAccessToken(session.access_token);
    await showGame();
  } catch (error) {
    authError.textContent = errorMessage(error);
  } finally {
    authSubmit.disabled = false;
  }
}

async function showGame(): Promise<void> {
  authPanel.hidden = true;
  gamePanel.hidden = false;
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
  await startAbyssGame();
  await refreshGame();
  if (!hasSession()) return;
  if (pollingTimer !== undefined) window.clearInterval(pollingTimer);
  pollingTimer = window.setInterval(() => void refreshGame(), 2500);
}

async function refreshGame(): Promise<void> {
  if (refreshInProgress || !hasSession()) return;
  refreshInProgress = true;
  expeditionError.textContent = "";
  try {
    const [creatureResult, latestExpedition, inventory] = await Promise.allSettled([
      api.getCreature(),
      api.getLatestExpedition(),
      api.getInventory(),
    ]);

    if (creatureResult.status === "fulfilled") {
      currentCreature = creatureResult.value;
      creatureSummary.textContent = `${currentCreature.name} · ${className(currentCreature.class)}`;
      creatureForm.hidden = true;
      creatureError.textContent = "";
    } else if (creatureResult.reason instanceof ApiError && creatureResult.reason.status === 404) {
      currentCreature = null;
      creatureSummary.textContent = "Todavía no tienes una criatura.";
      creatureForm.hidden = false;
    } else {
      throw creatureResult.reason;
    }

    if (latestExpedition.status === "fulfilled") {
      currentExpedition = latestExpedition.value;
    } else {
      throw latestExpedition.reason;
    }

    if (inventory.status === "fulfilled") {
      renderInventory(inventory.value);
    } else {
      throw inventory.reason;
    }

    renderExpeditionStatus(currentExpedition);
    expeditionSubmit.disabled = !currentCreature || currentExpedition?.expedition.status === "running";
    updateAbyss();
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      setAccessToken(null);
      showAuth();
      authError.textContent = "Tu sesión venció. Vuelve a entrar.";
    } else {
      expeditionError.textContent = errorMessage(error);
    }
  } finally {
    refreshInProgress = false;
  }
}

async function createCreature(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  const form = new FormData(creatureForm);
  const name = String(form.get("name") ?? "");
  const creatureClass = String(form.get("class") ?? "explorer") as CreatureClass;
  const button = creatureForm.querySelector<HTMLButtonElement>("button[type=submit]");
  if (button) button.disabled = true;
  creatureError.textContent = "";
  try {
    await api.createCreature(name, creatureClass);
    await refreshGame();
  } catch (error) {
    creatureError.textContent = errorMessage(error);
  } finally {
    if (button) button.disabled = false;
  }
}

async function sendExpedition(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  expeditionSubmit.disabled = true;
  expeditionError.textContent = "";
  const depth = Number(new FormData(expeditionForm).get("depth"));
  try {
    await api.startExpedition(depth);
    await refreshGame();
  } catch (error) {
    expeditionError.textContent = errorMessage(error);
    expeditionSubmit.disabled = false;
  }
}

async function logout(): Promise<void> {
  try {
    await api.logout();
  } catch {
    // La sesión se limpia localmente aunque el servidor ya la haya vencido.
  }
  setAccessToken(null);
  currentCreature = null;
  currentExpedition = null;
  showAuth();
}

function renderInventory(inventory: InventoryItem[]): void {
  inventoryList.replaceChildren();
  if (inventory.length === 0) {
    const empty = document.createElement("li");
    empty.className = "muted";
    empty.textContent = "Vacía por ahora";
    inventoryList.append(empty);
    return;
  }
  for (const item of inventory) {
    const entry = document.createElement("li");
    entry.textContent = `${item.name} × ${item.quantity}`;
    inventoryList.append(entry);
  }
}

function renderExpeditionStatus(details: ExpeditionDetails | null): void {
  if (!details) {
    expeditionStatus.textContent = "Todavía no hay expediciones.";
    return;
  }
  const expedition = details.expedition;
  if (expedition.status === "running") {
    const endTime = new Date(expedition.ends_at).toLocaleTimeString("es", {
      hour: "2-digit",
      minute: "2-digit",
    });
    expeditionStatus.textContent = `Tu criatura explora el piso ${expedition.depth}. Regreso previsto: ${endTime}.`;
  } else {
    const latestEvent = details.events.at(-1);
    expeditionStatus.textContent = latestEvent?.description ?? "La expedición terminó sin eventos.";
  }
}

function className(creatureClass: CreatureClass): string {
  const labels: Record<CreatureClass, string> = {
    explorer: "Exploradora",
    tank: "Tanque",
    hacker: "Hackeadora",
  };
  return labels[creatureClass];
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Ocurrió un error inesperado.";
}

authForm.addEventListener("submit", (event) => void authenticate(event));
authModeButton.addEventListener("click", () => setAuthMode(!isRegistering));
creatureForm.addEventListener("submit", (event) => void createCreature(event));
expeditionForm.addEventListener("submit", (event) => void sendExpedition(event));
logoutButton.addEventListener("click", () => void logout());

setAuthMode(false);
if (hasSession()) {
  void showGame();
} else {
  showAuth();
}
