# Full-Stack Rust Task Manager Demo

## 🚀 What we built

A complete full-stack Rust application with:

### Backend (Axum)
- REST API with CRUD operations for tasks
- In-memory task storage
- CORS support for frontend communication
- Static file serving for the frontend

### Frontend (Sauron)
- WebAssembly application
- Reactive UI with Elm architecture
- HTTP client for API communication
- Form handling and validation

### Shared
- Common types between frontend and backend
- Serialization/deserialization with serde

## 🏗️ Architecture

```
├── backend/           # Axum REST API server
├── frontend/          # Sauron WebAssembly frontend  
├── shared/            # Shared types and models
└── build.sh           # Build script
```

## 🔌 API Endpoints

- `GET /api/tasks` - Get all tasks
- `POST /api/tasks` - Create a new task
- `GET /api/tasks/:id` - Get a specific task
- `PUT /api/tasks/:id` - Update a task
- `DELETE /api/tasks/:id` - Delete a task

## 💻 Demo

1. **Build**: `./build.sh`
2. **Run**: `cargo run -p backend`
3. **Visit**: http://localhost:3000

### Features to try:
- ✅ Add new tasks with title and description
- ✅ Mark tasks as completed (checkbox)
- ✅ Edit tasks inline
- ✅ Delete tasks
- ✅ Real-time UI updates

## 🛠️ Tech Stack

- **Backend**: Rust + Axum + Tokio + Serde + UUID
- **Frontend**: Rust + Sauron + WebAssembly + web-sys
- **Build**: wasm-pack + Cargo workspace
- **HTTP**: Fetch API with CORS support

This demonstrates a modern, performant full-stack architecture entirely in Rust!