# Mercado del Abismo — Diseño técnico

Oct 30, 2026 · @Ruby

## Visión

Mercado del Abismo es un juego web persistente en pixel art 2D para unos 5 amigos en PC: mandas criaturas propias a un abismo infinito, vendes y fabricas lo que traen, y peleas jefes que cambian de género de juego.

El tono es kawaii-gótico en neón: criaturas pequeñas y tiernas en una ciudad cyberpunk bajo lluvia constante, con historias melancólicas. Bajo la ciudad hay un abismo hecho de recuerdos de gente que alguien "olvidó". En la entrada hay un mercado nocturno donde cada amigo tiene un puesto.

**Loop de juego**

1. Preparas una expedición: criatura, equipo y profundidad.
2. El abismo avanza solo mientras no estás y deja un diario de eventos (asíncrono).
3. Vuelves y revisas botín, heridas y recuerdos hallados.
4. Fabricas, vendes, compras o fías en el mercado del grupo.
5. Cuando aparece un jefe, quien esté conectado entra en tiempo real; cada jefe usa su propio modo de combate.

**Por qué funciona con 5 personas:** cada profundidad suelta materiales distintos y cada criatura se especializa, así que nadie tiene todo y la economía obliga a negociar. La historia se desbloquea por hitos del grupo (por ejemplo "entre todos llegaron al piso 10"), para que nadie se la spoilee a otro.

## Criaturas propias y personalización

Las criaturas son diseños originales, no personajes de ninguna franquicia, y se arman con **capas de partes** sobre un sprite base de 32×32 px. Cada capa es un PNG que se recolorea con una paleta, así que pocas imágenes dan muchas combinaciones.

| Capa | Ejemplos | Efecto en juego |
| --- | --- | --- |
| Cuerpo | polilla-bot, gato glitch, fantasmita, medusa de neón | Define la clase base y las estadísticas iniciales |
| Cabeza / orejas | antenas, orejas de gato, capucha | Cosmético, algunas dan bonus pequeño |
| Ojos | pixelados, visor, lágrima brillante | Cosmético |
| Accesorio | audífonos, gabardina, cadena, parche | Puede dar habilidad pasiva |
| Paleta | magenta/cian, violeta apagado, rojo sangre | Cosmético, desbloqueable por hitos |

Cada criatura guarda en PostgreSQL un JSON con las partes y la paleta elegidas, no una imagen. El cliente la dibuja en tiempo real con Canvas, lo que permite cambiar el aspecto sin regenerar nada y usar el mismo sistema en el mercado, el abismo y los jefes.

**Clases del MVP:** exploradora (encuentra más recuerdos), tanque (aguanta profundidad) y hackeadora (abre cofres y atajos). Cada clase cambia qué materiales trae.

## Arquitectura y stack

El backend Rust empieza con dos servicios desplegables en Docker Compose: una API REST y un worker que resolverá expediciones asíncronas. Comparten el paquete de código y PostgreSQL para mantener simple el MVP; no se separa cada dominio ni se añade Redis hasta que haga falta para el estado en vivo.

&#91;embedded content: arquitectura · 5 componentes del MVP, 2 posteriores\]

El MVP corre con navegador, Nginx, Rust, PostgreSQL y Redis; Rails y Neo4j entran en las fases 4 y 5.

| Capa | Tecnología | Por qué |
| --- | --- | --- |
| Cliente | TypeScript, Phaser 3, Vite | Escenas 2D y pixel art sin motor pesado; tu front ya es JS/TS |
| Servidor | Rust (Axum, Tokio, sqlx) | Es lo que quieres aprender y consume poca RAM |
| Datos | PostgreSQL | Transacciones para la economía y inventarios |
| Estado en vivo | Redis | Presencia, salas de jefes y temporizadores |
| Contenido | Ruby on Rails | Editor de historias, loot y balance sin recompilar Rust |
| Grafo | Neo4j | Recuerdos y cadenas de comercio; se agrega cuando el grafo aporta |

Neo4j consume cerca de 1 GB de RAM (estimación aproximada), por eso el lore vive en PostgreSQL durante el MVP y se migra en la fase 5.

## Esquema de base de datos

PostgreSQL es la fuente de verdad. Toda la economía va en transacciones para que nadie duplique objetos ni monedas. Los precios y saldos se guardan como enteros (centavos de la moneda del juego), nunca como decimales.

| Tabla | Campos clave | Para qué |
| --- | --- | --- |
| `players` | id, nombre, password\_hash, creado\_en | Cuentas de los amigos |
| `creatures` | id, player\_id, nombre, clase, partes (jsonb), paleta, nivel, estado | Criaturas personalizadas |
| `items` | id, nombre, tipo, rareza, receta\_id | Catálogo de objetos |
| `inventory` | player\_id, item\_id, cantidad | Inventario por jugador (PK compuesta) |
| `expeditions` | id, creature\_id, profundidad, inicia\_en, termina\_en, estado, semilla | Expediciones asíncronas |
| `expedition_events` | id, expedition\_id, tick, texto, loot (jsonb) | Diario del abismo |
| `recipes` | id, resultado\_item\_id, ingredientes (jsonb), tiempo\_s | Fabricación |
| `crafting_jobs` | id, player\_id, recipe\_id, termina\_en | Trabajos de fabricación en curso |
| `listings` | id, vendedor\_id, item\_id, cantidad, precio, estado | Ofertas del mercado |
| `trades` | id, comprador\_id, listing\_id, precio, creado\_en | Historial de ventas |
| `debts` | id, deudor\_id, acreedor\_id, monto, interes\_pct, vence\_en, estado | Fiado entre amigos |
| `market_prices` | item\_id, precio\_ref, oferta, demanda, actualizado\_en | Precio de referencia por objeto |
| `memories` | id, profundidad, orden, texto, requisito (jsonb) | Fragmentos de historia |
| `player_memories` | player\_id, memory\_id, desbloqueado\_en | Qué ha visto cada quien |
| `boss_encounters` | id, boss\_id, modo, estado, semilla, inicia\_en | Jefes activos |
| `boss_results` | encounter\_id, player\_id, puntaje, recompensa (jsonb) | Resultados y premios |

**Reglas de integridad:** el inventario no puede quedar negativo (`CHECK cantidad >= 0`), cada venta es una sola transacción que mueve objeto y dinero, y las expediciones se resuelven por un worker que lee `termina_en`, no por el cliente.

## Jefes dinámicos

Cada jefe declara su **modo de combate** y el motor carga el módulo correspondiente. Todos los modos comparten el mismo contrato, así que agregar un jefe nuevo es agregar un módulo y una fila de datos, no reescribir el juego.

| Modo | Cómo se juega | Dónde corre la lógica | Ejemplo de jefe |
| --- | --- | --- | --- |
| Turnos con timing | Eliges acción y pulsas en el momento justo | Servidor decide el resultado, cliente anima | El Archivista (ataques en compás) |
| Shooter / bullet hell | Esquivas patrones y disparas | Servidor simula a 20 ticks/s con semilla fija; cliente interpola | La Tormenta Roja |
| Plataformas | Saltas y escalas mientras el jefe cambia la sala | Cliente predice, servidor valida posiciones | El Coloso Oxidado |

**Contrato común** (una interfaz en Rust, implementada por cada modo):

1. `iniciar(semilla, jugadores)` crea el estado inicial.
2. `recibir_input(jugador, input)` valida y encola la acción.
3. `tick(dt)` avanza la simulación y devuelve eventos.
4. `estado_para(jugador)` devuelve lo que ese cliente debe dibujar.
5. `terminado()` indica victoria o derrota y calcula recompensas.

**Decisiones de diseño:**

- El servidor es autoritativo: el cliente manda inputs, no posiciones. Con 5 amigos no hace falta anti-trampas fuerte, pero esto evita desincronizaciones.
- Cada jefe vive en una "sala" de Redis con la lista de jugadores y el estado resumido; si alguien se desconecta, su criatura sigue en piloto automático.
- El patrón de cada jefe se describe en datos (JSON o RON) para ajustarlo desde el panel de Rails sin recompilar.
- En el MVP basta un jefe en un solo modo; el turno con timing es el más barato de construir, así que conviene empezar por ahí.

## Economía

La economía del MVP tiene cuatro verbos: fabricar, comprar, vender y deber. El contrabando queda como fase posterior, pero el diseño deja el gancho listo desde el inicio.

| Verbo | Cómo funciona | Regla clave |
| --- | --- | --- |
| Fabricar | Recetas que consumen materiales y tardan tiempo real | El trabajo se guarda con `termina_en` y se cobra al volver |
| Vender | Publicas una oferta con precio y cantidad | El objeto queda reservado hasta que se vende o se cancela |
| Comprar | Aceptas una oferta de otro jugador o vendes al mercado general | Una transacción mueve objeto y dinero a la vez |
| Deber | Fiado entre amigos con monto, interés y fecha límite | Si vence, sube el interés y baja la reputación |

**Precios dinámicos:** el precio de referencia de cada objeto sube cuando el grupo lo compra mucho y baja cuando abunda. Una regla simple basta para el MVP: cada hora, `precio_ref` se mueve un porcentaje pequeño según la relación oferta/demanda de las últimas 24 horas, con un tope para evitar picos.

**Reputación:** cada jugador tiene un valor que sube al pagar a tiempo y cumplir tratos, y baja al incumplir. Es la base del sistema de deudas y, más adelante, del contrabando.

**Gancho al contrabando (fase 5):** objetos marcados como `ilegal` que no aparecen en el mercado general, solo en un canal oculto entre jugadores, con riesgo de "redada" que confisca inventario. Para soportarlo desde ahora, `items` lleva una columna `legalidad` y `listings` una columna `canal` (público u oculto), aunque el MVP solo use el público.

**Anti-inflación:** el dinero entra por ventas al mercado general y recompensas de jefes, y sale por impuestos de venta, costos de fabricación y reparaciones. Con 5 jugadores conviene revisar el balance cada semana desde el panel de administración.

## Repositorio y despliegue

Un solo monorepo mantiene todo junto y simple de desplegar en tu servidor Ubuntu.

```text
mercado-del-abismo/
├── server/            # Rust (Axum + Tokio)
│   ├── src/api/       # rutas REST: cuentas, mercado, inventario
│   ├── src/ws/        # WebSockets: presencia y jefes
│   ├── src/sim/       # simulación del abismo y expediciones
│   ├── src/boss/      # contrato común + módulos por modo
│   ├── src/economy/   # precios, deudas, fabricación
│   └── migrations/    # SQL versionado (sqlx)
├── client/            # TypeScript + Phaser 3 + Vite
│   ├── src/scenes/    # Mercado, Puesto, Abismo, Jefe
│   ├── src/creature/  # renderer de capas y paletas
│   └── public/assets/ # sprites, música, fuentes
├── admin/             # Ruby on Rails (panel de contenido, fase 4)
├── content/           # historias, loot tables y jefes en JSON/RON
├── deploy/            # docker-compose, nginx, scripts de backup
└── docs/
```

**Despliegue en el servidor (8 GB de RAM, i7 de 3.ª generación):**

| Servicio | RAM aproximada | Nota |
| --- | --- | --- |
| Servidor Rust | 50–150 MB | Muy ligero |
| PostgreSQL | 300–600 MB | Con `shared_buffers` ajustado |
| Redis | 50–100 MB | Solo estado efímero |
| Nginx | 20–50 MB | HTTPS y archivos estáticos |
| Rails (fase 4) | 300–500 MB | Se levanta solo cuando se edita contenido |
| Neo4j (fase 5) | \~1 GB | Solo cuando el grafo aporte |

Estas cifras son estimaciones aproximadas, no mediciones. Aun con todo encendido queda margen para el sistema y tu servidor de Minecraft si lo compartes en la misma máquina, aunque conviene medir antes de sumarlo.

**Operación:** Docker Compose para todos los servicios, Nginx con Let's Encrypt para HTTPS, un cron con `pg_dump` diario a otro disco o servicio externo, y migraciones con `sqlx` para que el esquema evolucione sin perder datos.

## MVP, hoja de ruta y riesgos

El MVP es jugable cuando tus 5 amigos pueden entrar, mandar una criatura, vender lo que trae y pelear un jefe.

**Alcance del MVP**

- 5 cuentas, un puesto y una criatura personalizable por jugador.
- 3 profundidades con materiales distintos y 3 clases de criatura.
- Expediciones asíncronas con diario de eventos.
- Mercado con ofertas entre jugadores, fabricación básica y deudas simples.
- 1 jefe de turnos con timing, jugable en tiempo real.
- 10 recuerdos de historia desbloqueables.

**Hoja de ruta** (cada fase termina con algo que ya se puede probar)

1. **Núcleo sin gráficos:** API en Rust, PostgreSQL, cuentas, inventario y expediciones. Listo cuando una expedición se resuelve sola y se ve en un log.
2. **Cliente pixel art:** escenas de Mercado, Puesto y Abismo, y creatura con capas. Listo cuando un amigo juega desde el navegador.
3. **Economía:** ofertas, fabricación, precios dinámicos y deudas. Listo cuando dos jugadores se intercambian objetos sin errores de saldo.
4. **Jefe en vivo:** WebSockets, Redis y el primer jefe de turnos. Listo cuando 3 jugadores pelean a la vez.
5. **Contenido y más jefes:** panel Rails, jefe shooter y jefe de plataformas.
6. **Grafo y contrabando:** Neo4j para recuerdos y cadenas de comercio, canal oculto y redadas.

**Riesgos y cómo se mitigan**

| Riesgo | Mitigación |
| --- | --- |
| Alcance se infla (3 modos de jefe, Rails, Neo4j) | Un solo jefe en el MVP; Rails y Neo4j solo después de la fase 4 |
| Arte pixel art toma más tiempo que el código | Empezar con paquetes libres y siluetas simples, y reemplazar después |
| Balance de economía injusto entre amigos | Panel de ajuste y revisión semanal de saldos |
| Duplicación de objetos por errores de concurrencia | Transacciones y `CHECK` en la base de datos desde la fase 1 |
| RAM limitada del servidor | Medir cada fase y encender Neo4j y Rails solo cuando hagan falta |

**Decisiones abiertas**

- Nombre definitivo del juego.
- Moneda del juego y su estética.
- Si las criaturas pueden morir de forma permanente o solo quedar heridas.
