# AGENTS.md — Mercado del Abismo

Guía para agentes que trabajen en este repositorio. Responde en español. Antes de tomar decisiones de producto o arquitectura, lee `abyssmarket-design.md`; es la referencia de diseño, no una especificación de un sistema ya implementado.

## Estado del repositorio

- `server/` contiene API + worker Rust, migración `0001_core.sql`, registro/sesiones Argon2 y resolución inicial de expediciones. `client/` contiene el primer flujo web Phaser; Compose ejecuta PostgreSQL, migrator, API, worker y cliente Nginx. Aún faltan mercado, fabricación y jefes; solo hay una prueba integrada de fase 1.
- Es un juego web persistente para unos cinco amigos: expediciones asíncronas, mercado/fabricación y jefes en tiempo real, con estética pixel art kawaii-gótica. Prioriza un MVP jugable y cambios pequeños, revisables.
- La arquitectura y fases descritas son planes. Confírmalas contra el código/configuración cuando aparezcan. No adelantes fases sin que se pida.

## Stack y límites previstos

- Cliente: TypeScript, Phaser 3 y Vite, sin añadir frameworks de UI pesados. Servidor: Rust con Axum, Tokio y sqlx. PostgreSQL guarda el estado durable; Redis se reserva para estado efímero en vivo.
- La primera separación desplegable elegida es `api` + `expedition-worker`, compilados desde el paquete `server/` y con PostgreSQL compartido. `migrate` aplica migraciones antes de arrancar ambos; el worker procesa expediciones vencidas con `FOR UPDATE SKIP LOCKED`.
- Rails para herramientas de contenido y Neo4j para el grafo son componentes posteriores, no parte del MVP. El documento de diseño tiene numeración de fases inconsistente para esas incorporaciones; resuélvela antes de planificarlas.
- Mantén bajo el consumo de recursos del despliegue previsto (Ubuntu, 8 GB de RAM). Antes de añadir dependencias, servicios o lenguajes, explica su costo de RAM y mantenimiento y pide aprobación.
- `server/`, `client/` y `deploy/` ya existen; las demás carpetas del árbol de `abyssmarket-design.md` siguen siendo propuestas.

## Invariantes de arquitectura

- El servidor valida y resuelve el juego: el cliente envía inputs, no posiciones, saldos ni resultados.
- PostgreSQL es la fuente de verdad; Redis solo mantiene presencia, salas y temporizadores reemplazables. La pérdida de Redis no debe perder progreso durable.
- Economía: dinero y cantidades como enteros; cada transferencia de objetos y dinero ocurre en una sola transacción SQL; el inventario no puede quedar negativo.
- Expediciones: las resuelve un worker del servidor según `termina_en`, nunca el cliente. Guarda la semilla para poder reproducir y depurar expediciones y jefes.
- Jefes: conserva un contrato común (`iniciar`, `recibir_input`, `tick`, `estado_para`, `terminado`); modos nuevos deben ser módulos, no reescrituras del motor.
- Criaturas: persiste datos de capas y paleta, no imágenes; el cliente las dibuja. Mantén historias, recetas, loot y patrones en datos/configuración, no hardcodeados cuando el diseño los contempla como contenido.
- No implementes Rails, Neo4j ni contrabando antes de la fase correspondiente y sin solicitud explícita.

## Convenciones al implementar

- Rust: usa `cargo fmt` y `clippy`; evita `unwrap()`/`expect()` en rutas de producción. Usa migraciones versionadas y nunca edites una migración ya aplicada: crea otra.
- Rust: usa `thiserror` para errores de librerías internas y reserva `anyhow` para los bordes de la aplicación; no bloquees el runtime Tokio con trabajo pesado. Mantén tests unitarios junto al código y tests de integración para economía y expediciones.
- TypeScript: conserva `strict`; evita `any` salvo justificación. Mantén red y estado de API/WebSocket fuera de las escenas de Phaser; define una sola vez los tipos compartidos de API y WebSocket.
- Desde `client/`: `npm ci`, `npm run typecheck`, `npm run build`. Rollup se fija en `4.60.4`: `4.67.2` terminaba con `SIGILL` en el i7 de tercera generación de este entorno. No lo actualices sin probar el build en esa CPU.
- El dueño eligió nombres SQL en inglés para el esquema nuevo. Sigue los identificadores establecidos en migraciones; el documento mezcla idiomas, así que no renombres campos existentes por inferencia.
- Comandos verificados desde la raíz: `docker compose -f deploy/docker-compose.yml up --build --detach --wait --wait-timeout 90`; `docker compose -f deploy/docker-compose.yml config --quiet`. Compose aplica migraciones antes de iniciar API/worker.
- Compose publica API en `127.0.0.1:3000`, PostgreSQL en `127.0.0.1:5433` y cliente en `127.0.0.1:4173` por defecto; los puertos de host son configurables en `.env`.
- Rust no está instalado en el host. Verificación: dentro de `rust:1-bookworm`, instala `rustfmt` y `clippy`, luego ejecuta `cargo fmt --check` y `cargo clippy --locked --all-targets -- -D warnings`.
- Tests de integración: primero levanta Compose; después ejecuta `docker run --rm --network deploy_default --env DATABASE_URL=postgres://abyssmarket:local-dev-password@postgres:5432/abyssmarket --volume "$PWD/server:/app" --workdir /app rust:1-bookworm cargo test --locked`. `#[sqlx::test]` crea bases aisladas y requiere permiso `CREATEDB` en el usuario de test.
- El API confía en `X-Real-IP` para limitar autenticación porque el cliente Nginx es su proxy; el puerto del API se publica solo en loopback. Mantén ese límite si cambias el proxy o exposición de red.
- Antes de cerrar una tarea, ejecuta verificaciones pertinentes. Añade tests de economía y expediciones al implementar esas áreas.
- Commits pequeños, en español e imperativo; trabaja en una rama por funcionalidad y no hagas push directo a `main`.

## Seguridad y contenido

- Contraseñas con Argon2; nunca en texto plano ni en logs. Secretos en variables de entorno; no subas `.env` ni credenciales (mantén un `.env.example` sin secretos cuando haya configuración de entorno).
- Valida y limita inputs del cliente (tamaño, rango y frecuencia). Los cambios de datos destructivos requieren confirmación explícita.
- Mantén criaturas, mundo y personajes originales; no copies arte, nombres, marcas ni personajes de franquicias ni letras de canciones. El texto del juego va en español.
- Si se incorporan assets, usa licencias libres y registra cada licencia en `docs/assets-licencias.md`.
- Al implementar despliegue/operación, sigue el objetivo de bajo consumo en Ubuntu (8 GB de RAM) y configura backups diarios de PostgreSQL con `pg_dump`.

## Cómo colaborar

- El dueño conoce varios lenguajes y quiere aprender Rust, Neo4j y Ruby: explica en un par de líneas las decisiones de diseño no obvias, sin simplificar de más.
- Si una tarea es ambigua o cambia economía, jefes o esquema, pregunta antes de implementar. Prefiere cambios pequeños a refactors amplios.
- Si esta guía contradice `abyssmarket-design.md`, pregunta cuál referencia debe prevalecer antes de tomar una decisión de diseño.
