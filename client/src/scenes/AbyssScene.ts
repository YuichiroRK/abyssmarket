import Phaser from "phaser";

export interface AbyssView {
  creatureName: string | null;
  creatureClass: string | null;
  depth: number | null;
  status: "idle" | "running" | "completed";
}

const COLORS = {
  ink: 0x100d1d,
  panel: 0x191529,
  magenta: 0xff63d8,
  cyan: 0x68e8f2,
  violet: 0x8d72ff,
  white: 0xf4eefa,
};

export class AbyssScene extends Phaser.Scene {
  private view: AbyssView = {
    creatureName: null,
    creatureClass: null,
    depth: null,
    status: "idle",
  };
  private artwork?: Phaser.GameObjects.Graphics;
  private title?: Phaser.GameObjects.Text;
  private subtitle?: Phaser.GameObjects.Text;

  constructor() {
    super("AbyssScene");
  }

  create(): void {
    this.artwork = this.add.graphics();
    this.title = this.add.text(28, 24, "EL ABISMO", {
      color: "#ff63d8",
      fontFamily: "monospace",
      fontSize: "20px",
      fontStyle: "bold",
    });
    this.subtitle = this.add.text(30, 54, "lluvia sobre los recuerdos", {
      color: "#a89bb8",
      fontFamily: "monospace",
      fontSize: "12px",
    });
    this.redraw();
  }

  setView(view: AbyssView): void {
    this.view = view;
    this.redraw();
  }

  private redraw(): void {
    if (!this.artwork || !this.title || !this.subtitle) return;
    const graphics = this.artwork;
    graphics.clear();
    graphics.fillStyle(COLORS.ink, 1);
    graphics.fillRect(0, 0, 800, 480);

    graphics.fillStyle(COLORS.panel, 1);
    graphics.fillRect(0, 300, 800, 180);
    graphics.fillStyle(COLORS.violet, 0.18);
    graphics.fillRect(40, 230, 720, 4);
    graphics.fillStyle(COLORS.cyan, 0.16);
    graphics.fillRect(96, 270, 610, 3);

    for (let index = 0; index < 18; index += 1) {
      const x = (index * 97 + 41) % 790;
      const y = (index * 61 + 94) % 280;
      graphics.fillStyle(index % 3 === 0 ? COLORS.magenta : COLORS.cyan, 0.34);
      graphics.fillRect(x, y, 2, 12 + (index % 4) * 3);
    }

    const bodyColor = this.view.creatureClass === "tank" ? COLORS.cyan : COLORS.magenta;
    graphics.fillStyle(COLORS.violet, 0.28);
    graphics.fillEllipse(400, 326, 112, 22);
    graphics.fillStyle(bodyColor, 1);
    graphics.fillRect(372, 252, 56, 56);
    graphics.fillTriangle(372, 264, 364, 236, 389, 254);
    graphics.fillTriangle(428, 264, 436, 236, 411, 254);
    graphics.fillStyle(COLORS.white, 1);
    graphics.fillRect(384, 273, 8, 8);
    graphics.fillRect(408, 273, 8, 8);
    graphics.fillStyle(COLORS.ink, 1);
    graphics.fillRect(387, 276, 4, 4);
    graphics.fillRect(411, 276, 4, 4);
    graphics.fillStyle(COLORS.cyan, 0.8);
    graphics.fillRect(396, 294, 8, 3);

    const depthLabel = this.view.depth === null ? "PISO ?" : `PISO ${this.view.depth}`;
    const stateLabel = this.view.status === "running" ? "EXPEDICIÓN EN CURSO" :
      this.view.status === "completed" ? "EXPEDICIÓN COMPLETADA" : "ESPERANDO TU PRIMERA SEÑAL";
    this.title.setText(this.view.creatureName ? `${this.view.creatureName.toUpperCase()} · ${depthLabel}` : "EL ABISMO");
    this.subtitle.setText(stateLabel);
  }
}
