use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use redis::{AsyncCommands, Client};
use serde_json::json;
use shared::{CreateTaskRequest, Task, UpdateTaskRequest};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};
use uuid::Uuid;

type RedisPool = Arc<Client>;

#[tokio::main]
async fn main() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    let client = Client::open(redis_url).expect("Failed to connect to Redis");
    let pool = Arc::new(client);

    let app = Router::new()
        .route("/api/tasks", get(get_tasks).post(create_task))
        .route("/api/tasks/:id", get(get_task).put(update_task).delete(delete_task))
        .nest_service("/", ServeDir::new("frontend/dist"))
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    println!("Redis URL: {}", std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()));
    axum::serve(listener, app).await.unwrap();
}

async fn get_tasks(State(pool): State<RedisPool>) -> Result<Json<Vec<Task>>, StatusCode> {
    let mut conn = pool.get_async_connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let keys: Vec<String> = conn.keys("task:*").await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut tasks = Vec::new();
    
    for key in keys {
        let task_json: String = conn.get(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Ok(task) = serde_json::from_str::<Task>(&task_json) {
            tasks.push(task);
        }
    }
    
    Ok(Json(tasks))
}

async fn get_task(
    Path(id): Path<Uuid>,
    State(pool): State<RedisPool>,
) -> Result<Json<Task>, StatusCode> {
    let mut conn = pool.get_async_connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let key = format!("task:{}", id);
    let task_json: Option<String> = conn.get(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match task_json {
        Some(json) => {
            let task: Task = serde_json::from_str(&json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(task))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_task(
    State(pool): State<RedisPool>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let task = Task::new(payload.title, payload.description);
    let task_json = serde_json::to_string(&task).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut conn = pool.get_async_connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = format!("task:{}", task.id);
    
    conn.set(&key, &task_json).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(task))
}

async fn update_task(
    Path(id): Path<Uuid>,
    State(pool): State<RedisPool>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, StatusCode> {
    let mut conn = pool.get_async_connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = format!("task:{}", id);
    
    let task_json: Option<String> = conn.get(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match task_json {
        Some(json) => {
            let mut task: Task = serde_json::from_str(&json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            if let Some(title) = payload.title {
                task.title = title;
            }
            if let Some(description) = payload.description {
                task.description = description;
            }
            if let Some(completed) = payload.completed {
                task.completed = completed;
            }
            
            let updated_json = serde_json::to_string(&task).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            conn.set(&key, &updated_json).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            Ok(Json(task))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_task(
    Path(id): Path<Uuid>,
    State(pool): State<RedisPool>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut conn = pool.get_async_connection().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = format!("task:{}", id);
    
    let deleted: usize = conn.del(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if deleted > 0 {
        Ok(Json(json!({"message": "Task deleted successfully"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}