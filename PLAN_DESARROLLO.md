# Plan de desarrollo — Mercado del Abismo

**Estado:** fase 1 lista y cliente web inicial conectado · fase 2 aún requiere más escenas y personalización de criatura.

## Decisiones confirmadas

- Avanzar por entregas pequeñas, empezando por la fase 1.
- Desplegar API y worker de expediciones como dos servicios Rust en Docker Compose; PostgreSQL compartido. El worker resuelve expediciones vencidas; todavía no se separan servicios por dominio.
- Todo proceso de ejecución debe tener su servicio/contenedor Compose; el cliente se sirve en su contenedor Nginx.
- Usar nombres SQL en inglés y permitir registro abierto.
- Añadir Redis, Rails y Neo4j cuando lleguen sus necesidades. Python, Java y C# quedan fuera hasta definir una función concreta.
- Skill de trabajo adaptada para OpenCode en `.opencode/skills/abyssmarket-godmode/SKILL.md`; se carga al reiniciar OpenCode.

## Pasos y seguimiento

1. **Base Dockerizada del backend — completada**
   - [x] API Rust con `/healthz` y `/readyz`.
   - [x] Contenedores API, worker y PostgreSQL; imagen Rust multi-stage y volumen durable para PostgreSQL.
   - [x] Plantilla `.env.example`; PostgreSQL se publica en `127.0.0.1:5433` para evitar el puerto 5432 ocupado en este entorno.
   - [x] Documentar comandos y decisiones en `AGENTS.md` y este archivo.
   - [x] Ejecutar `fmt`, `clippy` y `cargo test` desde un contenedor Rust.
2. **Núcleo fase 1 — completado**
   - [x] Migración inicial en inglés: cuentas, sesiones, criaturas, inventario, expediciones, eventos y tablas de recompensas.
   - [x] Registro/login abierto con Argon2id, sesiones opacas con token hash, validación y rate limit por IP.
   - [x] API para crear una criatura y comenzar/consultar/cerrar sesión; máximo una expedición activa por criatura.
   - [x] Migrator Compose; worker reclama trabajos vencidos con `SKIP LOCKED` y en una transacción aplica botín, evento y estado final usando la semilla persistida.
   - [x] Tests unitarios y test integrado de registro, criatura, expedición, idempotencia y botín reproducible.
3. **Cliente Phaser — en progreso**
   - [x] Cliente TypeScript/Phaser/Vite con API y tipos fuera de las escenas; UI ligera en español.
   - [x] Flujo web de registro/login, creación de criatura, expedición, seguimiento e inventario.
   - [x] Cliente dockerizado con Nginx como servicio Compose y proxy de API same-origin.
   - [x] Cargar Phaser al entrar al juego para mantener ligera la pantalla de autenticación.
   - [ ] Añadir escenas de Mercado/Puesto y renderer de capas/paletas para completar la fase.
4. **Economía y juego en vivo — pendiente**
   - [ ] Añadir mercado/fabricación y luego jefes en vivo según la hoja de ruta; incorporar Redis cuando haga falta.
5. **Contenido y operación — pendiente**
   - [ ] Evaluar Rails, Neo4j, backups diarios y despliegue público en su fase correspondiente, midiendo recursos antes de sumar servicios.
6. **Publicación incremental — en marcha**
   - [x] Actualizar este archivo con resultados y pendientes reales de cada entrega.
   - [x] Ejecutar las verificaciones disponibles y corregir fallos para la base Dockerizada.
   - [x] Revisar el diff, crear commits pequeños en español en una rama de funcionalidad y subir esta entrega a GitHub.

## Publicación de la base Dockerizada

- Rama: `feat/base-dockerizada`.
- Commits iniciales: `3321dbe` (`Prepara servicios Rust en Docker`), `ace624b` (`Documenta plan y decisiones de arquitectura`) y `500d63d` (`Registra commits de la entrega inicial`).
- Push: completado a GitHub en `feat/base-dockerizada`; el historial de la rama registra los commits correspondientes.

## Publicación del núcleo fase 1

- Rama de trabajo: `feat/nucleo-fase-1`.
- Commits: `9cd83ab` (`Implementa núcleo inicial de fase 1`) y `af94718` (`Actualiza guía y adapta skill GodMode a OpenCode`).
- Push: completado a GitHub en `feat/nucleo-fase-1`; el historial remoto contiene ambos commits.

## Publicación del cliente fase 2

- Rama de trabajo: `feat/cliente-fase-2`.
- Estado: cliente inicial, verificación y Compose completados; escenas adicionales y renderer personalizable pendientes. Commits/push pendientes.

## Verificaciones realizadas

- `docker compose -f deploy/docker-compose.yml config --quiet` — pasó.
- `docker compose -f deploy/docker-compose.yml build` — pasó.
- `docker compose -f deploy/docker-compose.yml up --detach --wait --wait-timeout 90` — pasó tras mover el puerto del host a 5433.
- `GET /healthz` y `GET /readyz` — ambos respondieron `204` con PostgreSQL disponible.
- `docker compose -f deploy/docker-compose.yml up --build --detach --wait --wait-timeout 90` — pasó con migrator, API, worker y PostgreSQL.
- `GET /healthz` y `GET /readyz` — ambos respondieron `204` tras el arranque final.
- `cargo fmt --check` y `cargo clippy --locked --all-targets -- -D warnings` — pasaron en contenedor `rust:1-bookworm`.
- `cargo test --locked` — pasó: 5 tests unitarios y 1 test integrado contra una base PostgreSQL temporal.
- `npm ci`, `npm run typecheck`, `npm run build`, `npm audit` en `client/` — pasaron; auditoría sin vulnerabilidades.
- Compose sirve `/` con `200`, proxya `/healthz` y `/readyz` con `204`, y proxya `/api/creatures/me` con el `401` esperado sin token.
- Vite divide el cliente inicial (~7.6 KB gzip) del chunk Phaser (~1.2 MB, 332 KB gzip); build correcto.
