# Todo API

Simple application which I want to use as an opininated playground/template
for production ready apps/APIs. Will focus on

* Build, configuration
  * Easy setup and local development
  * 12-factor app style configuration
* Observability and telemetry
  * Good observability setup for smooth bugfinding + fixing
* Domain-driven design
  * Keep domain pure and testable, push effects to the "outer layers"
* Vertical slice architecture
  * Low coupling, but also focus on cohesion
* Cloud-native deployment
* ...

These are what I consider important factors when developing reliable, quality applications reasonably fast.

## Variants

* [Rust using Axum + sqlx + sqlite + tracing + opentelemetry...](/todo-api/rust)
