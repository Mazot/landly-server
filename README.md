# 🌍 Landly APP Server

[![Rust](https://img.shields.io/badge/rust-1.87+-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com)
[![PostgreSQL](https://img.shields.io/badge/postgresql-17-blue.svg)](https://www.postgresql.org)
[![Redis](https://img.shields.io/badge/redis-7-red.svg)](https://redis.io)
[![codecov](https://codecov.io/gh/Mazot/landly-server/branch/master/graph/badge.svg)](https://codecov.io/gh/Mazot/landly-server)

## About

Landly is a backend server for an application that helps people find businesses, helpers, and official representatives of different nationalities in foreign countries. Whether you're a traveler, expatriate, or immigrant, Landly connects you with culturally familiar services and support networks in your new location.

**Key Features:**

- 🏢 **Business Discovery** - Find businesses run by or catering to your nationality
- 🤝 **Community Helpers** - Connect with volunteers and community organizers
- 🏛️ **Official Representatives** - Locate embassies, consulates, and cultural centers
- 🌐 **Multi-language Support** - Browse content in your preferred language

## 🛠️ Technology Stack

### Core Technologies

- **[Rust](https://www.rust-lang.org)** - Systems programming language for performance and safety
- **[Actix Web](https://actix.rs)** - Powerful, pragmatic, and extremely fast web framework
- **[Diesel](https://diesel.rs)** - Safe, extensible ORM and Query Builder
- **[Tokio](https://tokio.rs)** - Asynchronous runtime for Rust

### Database & Caching

- **[PostgreSQL 17](https://www.postgresql.org)** - Advanced open source relational database
- **[Redis 7](https://redis.io)** - In-memory data structure store for caching

### Authentication & Security

- **[jsonwebtoken](https://crates.io/crates/jsonwebtoken)** - JWT implementation
- **[oauth2](https://crates.io/crates/oauth2)** - OAuth 2.0 client library
- **[bcrypt](https://crates.io/crates/bcrypt)** - Password hashing

### Development & Deployment

- **[Docker](https://www.docker.com)** - Containerization platform
- **[Docker Compose](https://docs.docker.com/compose/)** - Multi-container Docker applications

## 📋 Prerequisites

- **Rust 1.87+** - [Install Rust](https://rustup.rs/)
- **Docker & Docker Compose** - [Install Docker](https://docs.docker.com/get-docker/)
- **PostgreSQL 17** (if running locally)
- **Redis 7** (if running locally)

## 🚀 Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/Mazot/landly-server.git
cd landly-server
```

### 2. Environment Setup

Copy the example environment file and configure your settings:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:

```env
# Database
DATABASE_URL=postgres://landly-user:landly-password@localhost:5432/landly

# Redis
REDIS_URL=redis://redis:redis-pass@localhost:6379/
REDIS_USER=redis
REDIS_PASSWORD=redis-pass

# JWT
JWT_SECRET=your-super-secret-jwt-key
JWT_EXPIRATION=3600

# OAuth (Google)
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
OAUTH_GOOGLE_REDIRECT_URL=http://localhost:8080/api/user/oauth/google/callback

# Server
FRONTEND_ORIGIN=http://localhost:8080
HOST=0.0.0.0
PORT=8080
```

### 3. Run with Docker (Recommended)

```bash
# Start all services
docker compose up -d

# View logs
docker compose logs -f landly-server

# Stop services
docker compose down
```

### 4. Local Development Setup

If you prefer to run locally:

```bash
# Install Diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Run database migrations
diesel migration run

# Start the development server
cargo run
```

## 🔗 API Documentation

### Interactive API Documentation

The API comes with comprehensive OpenAPI 3.0 documentation accessible via Swagger UI:

```url
http://localhost:8080/swagger-ui
```

This interactive documentation provides:

- **Complete API reference** with all endpoints
- **Request/response schemas** with examples
- **Try it out functionality** to test endpoints directly
- **Authentication setup** for protected routes
- **Model definitions** for all data structures

### Authentication Endpoints

#### Traditional Auth

```http
POST /api/user/signup
Content-Type: application/json

{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "securepassword123"
}
```

```http
POST /api/user/signin
Content-Type: application/json

{
  "email": "john@example.com",
  "password": "securepassword123"
}
```

Signup v2 additionally accepts optional profile fields (`name`, `locale`, `here_as`, `home_country_id`, `avatar_color`) and a default corridor (`corridor_from_country_id`, `corridor_to_country_id`) — the user and the corridor are created in one transaction.

#### OAuth 2.0 (Google)

```http
# Initiate OAuth flow
GET /api/user/oauth/google/login

# OAuth callback (handled automatically)
GET /api/user/oauth/google/callback?code=...&state=...
```

### Profile

```http
# Current user profile with stats (places added, etc.)
GET /api/user/me
Authorization: Bearer <jwt_token>

# Update profile (name, bio, city, locale, here_as, ...)
PUT /api/user/me

# Update notification settings (free-form JSON object)
PUT /api/user/me/notifications
```

### Corridors

A corridor is the user's "from country → to country" pair the map opens to.

```http
POST   /api/corridor/create            # { from_country_id, to_country_id, is_default? }
GET    /api/corridor/list
PUT    /api/corridor/set-default/{id}
DELETE /api/corridor/delete/{id}
GET    /api/corridor/stats/{id}        # live-place counters by org type + "new this week"
```

All corridor endpoints require authentication and operate only on the caller's corridors.

### Organization Management

```http
# List organisations (only status=live)
GET /api/organisation/list

# Geo search for the map: bbox or origin+radius, filters and sorting
GET /api/organisation/search?min_lat=52&min_lng=13&max_lat=53&max_lng=14
GET /api/organisation/search?lat=52.5&lng=13.4&radius_km=25&sort=nearest
#   filters: types=embassy,community  open_now=true  languages=Russian,English
#            verified=true  min_rating=4.5  added_by=volunteer  cost=free
#   sort:    nearest | recent | verified

# Fetch one organisation (openNow computed from opening_hours + timezone)
GET /api/organisation/fetch/{id}

# Create organisation (requires auth; new submissions get status=pending)
POST /api/organisation/create
Authorization: Bearer <jwt_token>

# Update / delete — only the creator or a moderator/admin
PUT    /api/organisation/update/{id}
DELETE /api/organisation/delete/{id}

# Count a visit (public)
POST /api/organisation/visit/{id}
```

### Countries

```http
# All countries
GET /api/common/countries

# Country page payload: country + live-place breakdown by org type
GET /api/common/countries/{id}
```

## 🗄️ Database Schema

### Core Tables

- **chats** - Some chats links
- **users** - User accounts, authentication and profile (name, bio, city, home country, locale `en/ru/uk`, `here_as`, RBAC `role`, notification settings)
- **user_providers** - OAuth provider linkages
- **corridors** - User corridors (from country → to country, one default per user)
- **organisations** - Organization entities (v2: moderation `status`, `created_by` ownership, `verified`, contacts, `services[]`/`languages[]`, `opening_hours` JSONB + `timezone`, `cost`, Google import fields, visit/rating counters)
- **organisation_types** - Organization classifications with stable `slug` (canonical: `embassy`, `business`, `helper`, `community`, `volunteer`)
- **countries** - Country master data (+`currency`, `phone_code`, `top_cities`)
- **languages** - Language master data
- **countries_connections** - Country relationships
- **countries_to_languages** - Country-language mappings
- **users_to_languages** - User language preferences

## 🔧 Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Database Migrations

```bash
# Create new migration
diesel migration generate migration_name

# Run migrations
diesel migration run

# Rollback last migration
diesel migration revert
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for security vulnerabilities
cargo audit
```

## 📊 Monitoring & Health Checks

### Health Check Endpoint

```http
GET /api/healthcheck
```

### Service Health Checks

The application includes health checks for:

- Database connectivity
- Redis connectivity  
- Application readiness

## 🐳 Docker Configuration

### Services

- **landly-server**: Main application container
- **db**: PostgreSQL 17 database
- **redis**: Redis 7 cache server

### Production Deployment

For production deployment, consider:

1. **Security**: Use secrets management for sensitive data
2. **Scaling**: Use container orchestration
3. **Monitoring**: Add logging and metrics collection
4. **Backup**: Implement database backup strategies
5. **SSL/TLS**: Configure HTTPS termination

## 🤝 Contributing

1. Clone the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Ensure all tests pass (`cargo test`)
- Add tests for new features
- Update documentation as needed

## 📄 License

This project is licensed under the MIT License.

## 🙏 Acknowledgments

- [Actix Web](https://actix.rs) - Amazing web framework
- [Diesel](https://diesel.rs) - Excellent ORM
- [Rust Community](https://www.rust-lang.org/community) - For the incredible ecosystem

---

Made with ❤️ and 🦀 Rust
