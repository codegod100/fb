# Full-Stack Rust Task Manager

A demonstration of full-stack Rust development using:
- **Backend**: Axum web framework
- **Frontend**: Sauron WebAssembly framework
- **Shared**: Common types and models

## Features

- Create, read, update, and delete tasks
- Mark tasks as completed
- Edit task titles and descriptions
- Responsive web interface
- Real-time updates between frontend and backend

## Development

### Prerequisites

- Rust (latest stable)
- wasm-pack (`cargo install wasm-pack`)

### Building

```bash
./build.sh
```

### Running

```bash
# Start the server (serves both API and frontend)
cargo run --bin backend

# Or run in development mode
cargo run --bin backend
```

The application will be available at http://localhost:3000

## API Endpoints

- `GET /api/tasks` - Get all tasks
- `POST /api/tasks` - Create a new task
- `GET /api/tasks/:id` - Get a specific task
- `PUT /api/tasks/:id` - Update a task
- `DELETE /api/tasks/:id` - Delete a task

## Architecture

```
├── backend/           # Axum REST API server
├── frontend/          # Sauron WebAssembly frontend
├── shared/            # Shared types and models
└── build.sh           # Build script
```

The backend serves both the REST API at `/api/*` and the static frontend files at `/`.