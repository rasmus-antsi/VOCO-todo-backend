use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};

#[derive(Serialize, FromRow)]
struct Task {
    id: i64,
    user_id: i64,
    title: String,
    done: bool,
}

#[derive(Deserialize)]
struct NewTask {
    title: String,
}

#[derive(Deserialize)]
struct UpdateTask {
    done: bool,
}

enum AppError {
    NotFound,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            AppError::Db(e) => {
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}

async fn list_tasks(State(pool): State<PgPool>) -> Result<Json<Vec<Task>>, AppError> {
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT id, user_id, title, done FROM tasks WHERE user_id = 1 ORDER BY id",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(tasks))
}

async fn create_task(State(pool): State<PgPool>, Json(input): Json<NewTask>) -> Result<Json<Task>, AppError> {
	let task = sqlx::query_as::<_, Task>(
		"INSERT INTO tasks (user_id, title, done) VALUES (1, $1, false) RETURNING id, user_id, title, done",
	)
	.bind(input.title)
	.fetch_one(&pool)
	.await?;
	Ok(Json(task))
}

async fn update_task(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateTask>,
) -> Result<Json<Task>, AppError> {
    let task = sqlx::query_as::<_, Task>(
        "UPDATE tasks SET done = $1 WHERE id = $2 AND user_id = 1 RETURNING id, user_id, title, done",
    )
    .bind(input.done)
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(task))
}

async fn delete_task(
	State(pool): State<PgPool>,
	Path(id): Path<i64>,
) -> Result<Json<Task>, AppError> {
	let task = sqlx::query_as::<_, Task>(
		"DELETE FROM tasks WHERE id = $1 AND user_id = 1 RETURNING id, user_id, title, done",
	)
	.bind(id)
	.fetch_optional(&pool)
	.await?
	.ok_or(AppError::NotFound)?;
	Ok(Json(task))
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&url).await.unwrap();

    let api = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}", patch(update_task).delete(delete_task));

    let app = Router::new().nest("/api", api).with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
