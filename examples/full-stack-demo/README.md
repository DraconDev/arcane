# Full-Stack Demo

A comprehensive multi-service deployment example demonstrating Arcane's capabilities.

## 📦 Services (9 Total)

| Service    | Description                | Port  |
| ---------- | -------------------------- | ----- |
| Frontend   | Nginx serving static files | 80    |
| API        | Flask REST API             | 8080  |
| Worker     | Background job processor   | -     |
| PostgreSQL | Primary database           | 5432  |
| Redis      | Cache & sessions           | 6379  |
| RabbitMQ   | Message queue              | 15672 |
| Prometheus | Metrics collection         | 9090  |
| Grafana    | Dashboards                 | 3000  |

## 🔐 Secrets (45+)

-   Database credentials
-   Redis/RabbitMQ auth
-   Stripe, SendGrid, Twilio
-   AWS S3 credentials
-   OAuth (Google, GitHub, Discord)
-   AI APIs (OpenAI, Anthropic)
-   Monitoring (Sentry, Datadog)
-   And more...

## 🚀 Deploy with Arcane

```bash
cd examples/full-stack-demo

# Dry run to verify
arcane deploy --target micro1 --compose docker-compose.yaml --env micro1 --dry-run

# Live deploy
arcane deploy --target micro1 --compose docker-compose.yaml --env micro1
```

## 📡 Verify Deployment

```bash
# Stream logs
arcane logs micro1

# Check API
arcane exec micro1 -- curl localhost:8080/services
```
