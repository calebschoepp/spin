---
name: developing-spin-apps
description: Create, edit, build, and run Spin apps.
metadata:
  version: "0.1"
---

# Developing Spin Apps

## What is a Spin app?

A Spin application is one or more **WebAssembly components** wired together by a **manifest** (`spin.toml`). Each component is a small, sandboxed program that reacts to an external event — typically an incoming HTTP request (there is also a Redis trigger). Spin itself is the host: it listens for events, routes them to the right component, runs the Wasm, and returns the result. Components start on every request and stop when the handler returns — there is no long-lived process to manage.

Key concepts:

- **Manifest (`spin.toml`)** — declares the app name, trigger routes, component sources, build commands, and permissions (allowed hosts, key-value stores, etc.).
- **Component** — a single Wasm module built from Rust, TypeScript/JavaScript, Python, Go, or any language that compiles to WASI. Each component has its own trigger binding (e.g. an HTTP route).
- **Trigger** — the event type the app responds to. HTTP (`[[trigger.http]]`) is by far the most common.
- **Sandboxing** — components have no network/filesystem access by default. You must explicitly grant permissions via `allowed_outbound_hosts`, `files`, `key_value_stores`, etc. in the manifest.

## When to use this skill

Use this skill whenever the user needs to work with Spin — creating a new app, editing components, configuring the manifest, building, or running locally.

## Workflow

The typical lifecycle of a Spin app follows four stages: **scaffold → edit → build → run**.

### 1. Scaffold

Create a new app from a template:

```bash
spin new -t <template> <app-name>
```

Common templates: `http-rust`, `http-ts`, `http-js`, `http-py`, `http-go`.

To add another component to an existing app:

```bash
spin add -t <template> <component-name>
```

If the user does not specify a language, ask them which language they want to use.

Note: If the user is already working with an existing Spin app then this step is not necessary.

### 2. Edit

- **`spin.toml`** — configure routes, set `allowed_outbound_hosts` for any external services the component needs to reach, grant `key_value_stores` or `sqlite_databases` access, and list any static `files` to mount. See the **manifest reference** for all fields.
- **Component source code** — implement the request handler. Each language SDK has its own conventions; consult the appropriate language reference below.

#### Outbound networking

Components are sandboxed. To make outbound HTTP calls, connect to Redis/MySQL/Postgres, or reach any external address, add entries to `allowed_outbound_hosts`:

```toml
[component.my-component]
allowed_outbound_hosts = ["https://api.example.com", "redis://localhost:6379"]
```

Use `"*://*:*"` only during development to allow all hosts.

#### Key-value and SQLite storage

Spin provides built-in key-value and SQLite storage. Grant access in the manifest:

```toml
[component.my-component]
key_value_stores = ["default"]
sqlite_databases = ["default"]
```

Then use the SDK's key-value or SQLite APIs in your component code. See each language reference for examples.

### 3. Build

Compile all components to Wasm:

```bash
spin build
```

This runs each component's `[component.<id>.build]` command defined in the manifest.

### 4. Run locally

Start the app on your machine:

```bash
spin up
```

The app is available at `http://127.0.0.1:3000` by default.

For a faster development loop, use **watch mode** — it rebuilds and restarts the app automatically when source files change:

```bash
spin watch
```

## References

Read these files for detailed guidance on specific topics. Always read the relevant reference **before** generating or editing code.

- [references/manifest.md](references/manifest.md) — When creating or editing `spin.toml`: routes, triggers, permissions, variables, component config, build settings.
- [references/cli.md](references/cli.md) — When you need the exact flags/options for any `spin` subcommand (`new`, `build`, `up`, `deploy`, `watch`, `add`, `doctor`, `plugins`, `templates`, `registry`).
- [references/rust-components.md](references/rust-components.md) — When building a component in **Rust**: project setup, HTTP handlers, outbound HTTP, key-value, SQLite, Redis, AI inferencing, external crates.
- [references/javascript-components.md](references/javascript-components.md) — When building a component in **JavaScript or TypeScript**: project setup, HTTP handlers, outbound HTTP, key-value, SQLite, NPM libraries, Node.js compat.
- [references/python-components.md](references/python-components.md) — When building a component in **Python**: project setup, virtual environments, HTTP handlers, outbound HTTP, key-value, SQLite, AI inferencing.
- [references/go-components.md](references/go-components.md) — When building a component in **Go** (TinyGo): project setup, HTTP handlers, outbound HTTP, key-value, SQLite, AI inferencing.
