---
name: abyssmarket-godmode
description: Usa cuando el usuario pida una nueva funcionalidad, corrección, cambio de API o esquema, investigación o publicación; aplica un flujo ligero de diseño, implementación, pruebas y documentación para Mercado del Abismo.
license: MIT
---

# Flujo de desarrollo para Mercado del Abismo

Adaptación para OpenCode de [CC_GodMode de cubetribe](https://github.com/cubetribe/openclaw-godmode-skill). Conserva su ciclo de trabajo, ajustado a las herramientas y reglas reales de este repositorio.

## Flujo

1. Clasifica la petición (funcionalidad, error, API/esquema, investigación u operación) y revisa `AGENTS.md`, `abyssmarket-design.md` y el código/configuración afectados.
2. Confirma contratos y decisiones contra fuentes ejecutables. Si una decisión de producto, economía, jefes o esquema no está resuelta, pregunta en un solo lote antes de fijarla.
3. Divide tareas de varias partes en pasos concretos y actualiza `PLAN_DESARROLLO.md` cuando cambie el progreso.
4. Implementa el cambio mínimo coherente. Para API/esquema, identifica rutas, clientes y consultas afectadas; añade migraciones nuevas en lugar de editar las ya aplicadas.
5. Ejecuta verificaciones específicas y las comprobaciones requeridas por `AGENTS.md`. No declares validaciones que no se hayan ejecutado; anota bloqueos y tests aún inexistentes.
6. Revisa el diff y el estado de Git, limita el commit a los archivos de la tarea y documenta el resultado. Usa una rama de funcionalidad; publica solo cuando el usuario lo haya pedido.

## Reglas propias del proyecto

- El servidor valida y resuelve el juego; PostgreSQL conserva el estado durable y Redis solo estado reemplazable.
- Mantén importes y cantidades enteros, transferencias económicas transaccionales y resolución de expediciones en el worker con semilla persistida.
- Mantén las criaturas como datos de capas/paleta y contenido como datos cuando lo contemple el diseño.
- Evita añadir lenguajes, servicios o dependencias sin explicar su costo de RAM y mantenimiento y obtener aprobación, según `AGENTS.md`.

## Adaptación a OpenCode

- Usa las herramientas y agentes que estén disponibles en la sesión. No asumas que existen los ocho agentes, MCP, Claude Code, OpenClaw, Playwright ni comandos `gh` descritos por la skill original.
- No delegues ni invoques subagentes salvo que el usuario o las instrucciones activas lo pidan explícitamente.
- No exijas `VERSION`, `CHANGELOG.md`, reportes por versión ni capturas de pantalla: este repositorio aún no los define. Añádelos solo si el usuario o el proyecto los solicita.
- Para GitHub, usa las herramientas disponibles y confirma el acceso antes de afirmar que se creó un PR, release o cambio de configuración.
