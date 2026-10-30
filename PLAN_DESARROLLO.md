# Plan de desarrollo — Mercado del Abismo

**Estado:** núcleo inicial de fase 1 implementado · sigue pendiente el cliente jugable.

## Decisiones confirmadas

- Avanzar por entregas pequeñas, empezando por la fase 1.
- Desplegar API y worker de expediciones como dos servicios Rust en Docker Compose; PostgreSQL compartido. El worker resuelve expediciones vencidas; todavía no se separan servicios por dominio.
- Todo proceso de ejecución que se incorpore debe tener su servicio/contenedor Compose; el cliente también se dockerizará cuando se añada.
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
3. **Cliente Phaser — pendiente**
   - [ ] Iniciar cliente TypeScript/Phaser/Vite en su contenedor Compose y conectar la primera experiencia jugable al servidor.
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
- Estado: implementación y verificaciones completadas; commits/push pendientes.

## Verificaciones realizadas

- `docker compose -f deploy/docker-compose.yml config --quiet` — pasó.
- `docker compose -f deploy/docker-compose.yml build` — pasó.
- `docker compose -f deploy/docker-compose.yml up --detach --wait --wait-timeout 90` — pasó tras mover el puerto del host a 5433.
- `GET /healthz` y `GET /readyz` — ambos respondieron `204` con PostgreSQL disponible.
- `docker compose -f deploy/docker-compose.yml up --build --detach --wait --wait-timeout 90` — pasó con migrator, API, worker y PostgreSQL.
- `GET /healthz` y `GET /readyz` — ambos respondieron `204` tras el arranque final.
- `cargo fmt --check` y `cargo clippy --locked --all-targets -- -D warnings` — pasaron en contenedor `rust:1-bookworm`.
- `cargo test --locked` — pasó: 5 tests unitarios y 1 test integrado contra una base PostgreSQL temporal.
