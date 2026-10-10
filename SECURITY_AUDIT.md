# Security audit — landly-server

Дата: 2026-10-10 · Ветка: `claude/app-security-audit-6uhdb5` (база `c98fd37`)
Метод: ручной разбор всего `src/` (auth-middleware, все 12 фич, utils, error), миграций,
Docker/compose, CI; `cargo audit` по `Cargo.lock`. Код не менялся — только этот отчёт.

## Сводка

| # | Severity | Находка | Где |
|---|---|---|---|
| 1 | **CRITICAL** | Логин принимает любой пароль — результат `bcrypt::verify` игнорируется | `user/entities.rs:273` |
| 2 | **HIGH** | Google OAuth привязывает аккаунт по email без проверки `email_verified` → захват аккаунта (в т.ч. pre-hijack) | `user/repositories.rs:136-150` |
| 3 | **HIGH** | Обход модерации и согласия для `person`: автор получает claim-токен и сам подтверждает `pending`-персону | `person/entities.rs:333`, `person/usecases.rs:89` |
| 4 | **HIGH** | Любой пользователь загружает фото в **любую** организацию и делает его главным | `images/usecases.rs:46`, `images/controllers.rs:28` |
| 5 | **HIGH** (deploy) | docker-compose публикует Postgres/Redis/MinIO на `0.0.0.0` с дефолтными паролями; бакет public | `docker-compose.yml` |
| 6 | MEDIUM | Редактирование `live`-организации автором не возвращает её в модерацию | `organisation/usecases.rs:140` |
| 7 | MEDIUM | OAuth `state` не привязан к браузеру → login CSRF | `user/controllers.rs:251`, `oauth/google.rs` |
| 8 | MEDIUM | JWT передаётся во фронт в query string редиректа | `user/controllers.rs:294` |
| 9 | MEDIUM | Нет rate limiting нигде (signin, signup, OAuth, отзывы, чек-ины, репорты, visit) | глобально |
| 10 | MEDIUM | Нет верификации email и политики паролей при signup | `user/usecases.rs` |
| 11 | MEDIUM | Перечисление пользователей: 404 на неизвестный email при signin; 422 с `Key (email)=(…) already exists` при signup | `error.rs:79`, `User::signin` |
| 12 | LOW | `GET /person/fetch/{id}` и `GET /organisation/fetch/{id}` отдают `pending`/`declined`/`rejected` записи | `person/entities.rs:215`, `organisation/entities.rs:175` |
| 13 | LOW | CORS: остаток из примера — разрешены все `*.rust-lang.org`; дефолт `FRONTEND_ORIGIN="*"` | `middlewares/cors.rs:11` |
| 14 | LOW | Нет отзыва JWT (logout / смена пароля), HS256 с секретом без проверки длины; в `.env.example` секрет `0987654321` | `utils/token.rs` |
| 15 | LOW | Спам/накрутка: чек-ины без лимита на пользователя, visit-counter без дедупа, отзыв на собственную организацию/себя | `organisation`, `review` |
| 16 | LOW | Загрузка: тип файла берётся из заголовка клиента, magic bytes не проверяются | `images/usecases.rs` |
| — | MEDIUM | `cargo audit`: 21 advisory в транзитивных зависимостях (aws-lc-sys, rustls, rustls-webpki, h2, bytes, diesel, …) — лечится `cargo update` | `Cargo.lock` |
| 17 | INFO | Unpinned образы (`rust:latest`, `debian:latest`, `minio:latest`), GitHub Actions по мажорным тегам | `Dockerfile`, workflows |

Что сделано хорошо: все SQL через Diesel DSL (инъекций нет), user id всегда из JWT-extension,
RBAC проверяется в usecase-ах и роль читается из БД на каждый запрос (повышение/понижение роли
действует сразу), владение для corridor/saved/review/org/images-delete проверяется, пагинация
клампится, ошибки БД не утекают (кроме constraint-деталей, см. №11), `path_matches` при обходе
через percent-encoding всё равно падает в `caller_user_id` → 401 (fail-closed), контакты персон
гейтятся в одном месте (`PersonContent::from_gated`), PKCE в OAuth, загрузка ограничена 10 MB
потоково.

---

## 1. CRITICAL — аутентификация по любому паролю

```rust
// src/app/features/user/entities.rs:273
hash::verify(password, &user_by_email.password_hash)?;
let token = user_by_email.generate_token()?;
```

`bcrypt::verify` возвращает `Ok(false)` для неверного пароля (это прямо зафиксировано тестом
`utils/hash.rs::test_verify_password_wrong_password`). `?` пробрасывает только ошибку, `bool`
отбрасывается — JWT выдаётся всегда.

**Эксплуатация:** `POST /api/user/signin {"email":"<любой существующий>","password":"x"}` →
валидный токен. Достаточно знать email админа/модератора → полный контроль над модерацией,
справочниками, удалением любых организаций и отзывов. Email-ы перечисляются (№11).

**Фикс:**
```rust
if !hash::verify(password, &user_by_email.password_hash)? {
    return Err(AppError::Unauthorized(json!({ "error": "Invalid email or password" })));
}
```
+ тот же ответ 401 для несуществующего email (не 404), + регрессионный тест. После фикса —
ротировать `JWT_SECRET` (инвалидирует все выданные токены) и проверить логи signin.

## 2. HIGH — захват аккаунта через Google OAuth

`upsert_oauth_user`: если связки provider+sub нет, ищется пользователь **по email** и к нему
привязывается Google-аккаунт; `GoogleUserInfo` не читает `email_verified`.

- Google-аккаунт может иметь неподтверждённый сторонний email → вход в чужой аккаунт.
- Обратный сценарий (pre-account takeover): атакующий регистрирует `victim@…` паролем
  (email не верифицируется, №10); жертва потом входит через Google и попадает в аккаунт,
  пароль от которого знает атакующий.

**Фикс:** требовать `email_verified == true`; автопривязку к существующему парольному аккаунту
делать только после подтверждения владения (вход паролем / письмо), либо создавать отдельный
аккаунт. Email сравнивать в нормализованном виде (lowercase).

## 3. HIGH — обход модерации/согласия для персон

`POST /person/create` возвращает автору `claimUrl` с токеном. `resolve_claim` принимает статус
`Pending` (а не только `Awaiting`). Значит автор сам вызывает
`POST /person/claim/{token}/confirm {"show_email":true,"show_whatsapp":true}` — персона сразу
становится `confirmed` (или `claimed` на автора, если передан его Bearer), минуя модерацию и
реального человека. Итог: публикация чужих контактов без согласия и выдача себя за другого.

**Фикс:** в `resolve_claim` разрешать только `Awaiting`; запрещать `claimed_by == recommended_by`;
не отдавать токен автору до одобрения модератором (идеально — отправлять ссылку сервером),
хранить в БД хэш токена.

## 4. HIGH — фото в чужие организации

`upload_image` не проверяет, что вызывающий — владелец организации (или модератор), и принимает
`is_primary=true`; `set_primary_image` проверяет только `uploaded_by`. Любой зарегистрированный
пользователь может заменить главное фото любой организации, а владелец/модератор не может его
удалить (`delete_image` разрешён только загрузившему).

**Фикс:** в upload/set-primary/delete применять тот же `ensure_can_manage` (creator или
moderator+), что и для организации; удаление — также модератору.

## 5. HIGH (deploy) — сервисы наружу с дефолтными паролями

В `docker-compose.yml` порты `5432`, `6379`, `9000`, `9001` публикуются без адреса (= `0.0.0.0`),
пароли по умолчанию `landly-password` / `redis-pass` / `minioadmin`, `S3_ACCESS_KEY_ID=minioadmin`.
Сервер же — на `127.0.0.1`. Если compose используется на проде как есть — БД (с хэшами паролей и
контактами персон) и Redis доступны из интернета.

**Фикс:** убрать `ports` у db/redis/minio (или `127.0.0.1:`), убрать дефолты паролей
(`${VAR:?required}`), отдельный ключ MinIO с правами только на бакет вместо root.

## 6. MEDIUM — редактирование после одобрения

`update_organisation` не трогает `status`: после одобрения автор может переписать название,
сайт, телефон, координаты — на ленте это появится без модерации (фишинг/спам). **Фикс:** при
изменении автором (не модератором) содержательных полей возвращать `status='pending'` или
писать правку в модерационную очередь.

## 7. MEDIUM — OAuth login CSRF

CSRF `state` хранится только в Redis (`oauth:google:pkce:<state>`), браузеру ничего не выдаётся,
`_state` в `oauth_google_login` отбрасывается. Атакующий начинает flow сам и подсовывает жертве
ссылку на callback со своим `code`+`state` → жертва залогинена в аккаунт атакующего.
**Фикс:** ставить HttpOnly/SameSite=Lax cookie со `state` и сверять в callback.

## 8. MEDIUM — JWT в URL

`/auth/google/callback?token=…&email=…` — токен оседает в истории браузера, логах прокси/CDN,
может уйти в `Referer`. **Фикс:** одноразовый короткоживущий код в URL, обмениваемый на токен
POST-запросом, или fragment (`#token=`) + немедленная очистка на фронте.

## 9. MEDIUM — нет rate limiting

Нигде нет ограничения частоты. Вместе с №1 это особенно опасно, но и после фикса: брутфорс
signin, массовая регистрация, накрутка отзывов/чек-инов/visit, спам репортами, bcrypt на signup
как CPU-DoS. **Фикс:** `actix-governor` (или лимит на reverse-proxy) — строго на
`/user/signin`, `/user/signup`, `/user/oauth/*`, мягче на мутирующие эндпоинты.

## 10. MEDIUM — signup без верификации и политики паролей

Пустой пароль принимается, email не подтверждается и не нормализуется (`A@x.com` и `a@x.com` —
разные аккаунты), длины `username`/`bio`/`name` не ограничены. bcrypt молча обрезает пароль до
72 байт. **Фикс:** минимальная длина пароля (≥ 8–10), lowercase email + уникальность по нему,
подтверждение email, лимиты длины полей.

## 11. MEDIUM — перечисление пользователей

- `signin` с неизвестным email → `DieselError::NotFound` → **404**, с известным → другой ответ.
- `signup` → 422 с `info.details()` Postgres: `Key (email)=(a@b.c) already exists.` (утечка
  имён колонок/значений из всех constraint-ов).

**Фикс:** единый 401 для signin; для constraint-ошибок отдавать обобщённое сообщение
(«email already registered») без `details()`.

## 12–17. LOW / INFO

- **12.** `fetch_person` / `fetch_organisation` по id не фильтруют статус: имя, био, город
  отклонённой (`declined`) персоны, которая отказалась от публикации, доступны по UUID. Отдавать
  404 для непубличных статусов (кроме автора/модератора).
- **13.** `allowed_origin_fn(... ends_with(".rust-lang.org"))` — копипаст из документации
  actix-cors, удалить. Дефолт `FRONTEND_ORIGIN="*"` заменить на обязательную переменную.
- **14.** JWT: нет revocation/`jti`, нет logout, нет проверки длины `JWT_SECRET` (в примере
  10 цифр). Требовать ≥ 32 байт случайного секрета при старте.
- **15.** `org_checkins` без уникальности/лимита на пользователя и организацию; `visit` —
  публичный счётчик без дедупа; можно оставить отзыв на собственную организацию / на себя как
  claimed-персону. `reports` без дедупа.
- **16.** Тип изображения доверяется заголовку клиента; бакет публичный → хостинг произвольных
  байт под видом картинок. Проверять magic bytes (`infer`/`image`), опционально перекодировать.
- **17.** Образы `rust:latest`, `debian:latest`, `quay.io/minio/aistor/minio:latest`,
  `minio/mc:latest` — пиннить по версии/digest; Actions пиннить по SHA.

Мелочи без отдельного номера: LIKE-шаблоны не экранируют `%`/`_` (не инъекция, только
неточный поиск/нагрузка); `make_common_get_request_cache` паникует без `REDIS_URL` (доступность,
противоречит «Redis опционален»); `SKIP_AUTH_ROUTES` — мёртвый код; пароль Redis виден в
`ps` контейнера (через `--requirepass`).

## cargo audit

`cargo audit` (v0.22.2, база RustSec на 2026-10-10) по корневому `Cargo.lock`:
**21 уязвимость + 12 warnings (unsound/yanked)**, все — транзитивные и исправляются
`cargo update` (прямые версии в `Cargo.toml` менять не нужно, кроме diesel → `>=2.3.8`).

| Крейт | Версия | Advisory | Суть | Фикс | Релевантность |
|---|---|---|---|---|---|
| aws-lc-sys | 0.37.0 | RUSTSEC-2026-0044/45/46/47/48 | обходы валидации X.509/PKCS7, timing в AES-CCM | ≥ 0.39.0 | TLS-стек aws-sdk-s3 → средняя |
| rustls | 0.23.31 | RUSTSEC-2026-0285 | TLS 1.3 handshake across encryption levels | ≥ 0.23.45 | исходящий TLS (S3, Google OAuth) |
| rustls-webpki | 0.101.7, 0.103.4 | RUSTSEC-2026-0049/98/99/104 | name constraints, CRL, panic | ≥ 0.103.13 | исходящий TLS; 0.101 — через старый стек aws-sdk |
| h2 | 0.3.26, 0.4.12 | RUSTSEC-2026-0258 | неограниченные пустые DATA-фреймы (DoS) | ≥ 0.4.16 | клиентский HTTP/2 |
| quinn-proto | 0.11.12 | RUSTSEC-2026-0037/0185 | DoS / memory exhaustion | ≥ 0.11.15 | через reqwest, низкая |
| bytes | 1.10.1 | RUSTSEC-2026-0007 | overflow в `BytesMut::reserve` | ≥ 1.11.1 | везде (actix), обновить |
| time | 0.3.41 | RUSTSEC-2026-0009 | stack exhaustion DoS | ≥ 0.3.47 | низкая |
| diesel | 2.2.10 | RUSTSEC-2026-0136/0137 (+ unsound 0111/0134/0172) | command injection в `COPY FROM/TO`, SQLite/MySQL-баги | ≥ 2.3.8 | COPY и SQLite/MySQL не используются — низкая, но обновить |

Warnings: unsound в `lru 0.16.3`, `rand 0.8.5/0.9.1`, `scc 2.4.0`, yanked `spin 0.10.0`.

То, что `audit.yml` (еженедельный) и `dependencies.yml` (еженедельный `cargo update` PR)
существуют, но lockfile так устарел, значит их PR/алерты не мержатся — стоит проверить.
`scripts/crates/Cargo.lock` не проверялся (вне рантайма сервера).

## Приоритет исправлений

1. №1 — немедленно (hotfix + ротация `JWT_SECRET` + проверка, не было ли входов в
   admin/moderator-аккаунты).
2. №2, №3, №4, №5, `cargo update`.
3. №6–№11.
4. Остальное — в бэклог.
