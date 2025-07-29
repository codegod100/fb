use sauron::{
    html::{attributes::*, *},
    prelude::*,
};
use shared::{CreateTaskRequest, Task, UpdateTaskRequest};
use uuid::Uuid;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{console, Request, RequestInit, Response};

#[derive(Debug, Clone)]
pub enum Msg {
    LoadTasks,
    TasksLoaded(Vec<Task>),
    SetNewTaskTitle(String),
    SetNewTaskDescription(String),
    CreateTask,
    TaskCreated(Task),
    ToggleTask(Uuid),
    TaskUpdated(Task),
    DeleteTask(Uuid),
    TaskDeleted(Uuid),
    EditTask(Uuid),
    SetEditTitle(String),
    SetEditDescription(String),
    SaveEdit(Uuid),
    TaskSaved(Task),
    CancelEdit,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Model {
    tasks: Vec<Task>,
    new_task_title: String,
    new_task_description: String,
    editing_task: Option<Uuid>,
    edit_title: String,
    edit_description: String,
    loading: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            new_task_title: String::new(),
            new_task_description: String::new(),
            editing_task: None,
            edit_title: String::new(),
            edit_description: String::new(),
            loading: false,
        }
    }
}

impl Application for Model {
    type MSG = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        Cmd::new(async { Msg::LoadTasks })
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::LoadTasks => {
                self.loading = true;
                Cmd::new(async {
                    match fetch_tasks().await {
                        Ok(tasks) => Msg::TasksLoaded(tasks),
                        Err(e) => Msg::Error(e),
                    }
                })
            }
            Msg::TasksLoaded(tasks) => {
                self.tasks = tasks;
                self.loading = false;
                Cmd::none()
            }
            Msg::SetNewTaskTitle(task_title) => {
                self.new_task_title = task_title;
                Cmd::none()
            }
            Msg::SetNewTaskDescription(description) => {
                self.new_task_description = description;
                Cmd::none()
            }
            Msg::CreateTask => {
                let task_title = self.new_task_title.clone();
                let description = self.new_task_description.clone();
                
                if task_title.trim().is_empty() {
                    return Cmd::none();
                }
                
                self.new_task_title.clear();
                self.new_task_description.clear();
                
                Cmd::new(async move {
                    match create_task(task_title, description).await {
                        Ok(task) => Msg::TaskCreated(task),
                        Err(e) => Msg::Error(e),
                    }
                })
            }
            Msg::TaskCreated(task) => {
                self.tasks.push(task);
                Cmd::none()
            }
            Msg::ToggleTask(id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == id) {
                    let completed = !task.completed;
                    Cmd::new(async move {
                        match update_task(id, None, None, Some(completed)).await {
                            Ok(task) => Msg::TaskUpdated(task),
                            Err(e) => Msg::Error(e),
                        }
                    })
                } else {
                    Cmd::none()
                }
            }
            Msg::TaskUpdated(updated_task) => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == updated_task.id) {
                    *task = updated_task;
                }
                Cmd::none()
            }
            Msg::TaskSaved(saved_task) => {
                // Update the task in the list
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == saved_task.id) {
                    *task = saved_task;
                }
                // Clear edit fields (edit mode was already exited in SaveEdit)
                self.edit_title.clear();
                self.edit_description.clear();
                Cmd::none()
            }
            Msg::DeleteTask(id) => {
                Cmd::new(async move {
                    match delete_task(id).await {
                        Ok(_) => Msg::TaskDeleted(id),
                        Err(e) => Msg::Error(e),
                    }
                })
            }
            Msg::TaskDeleted(id) => {
                self.tasks.retain(|t| t.id != id);
                Cmd::none()
            }
            Msg::EditTask(id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == id) {
                    self.editing_task = Some(id);
                    self.edit_title = task.title.clone();
                    self.edit_description = task.description.clone();
                }
                Cmd::none()
            }
            Msg::SetEditTitle(task_title) => {
                self.edit_title = task_title;
                Cmd::none()
            }
            Msg::SetEditDescription(description) => {
                self.edit_description = description;
                Cmd::none()
            }
            Msg::SaveEdit(id) => {
                // Guard: only save if we're actually editing this task
                if self.editing_task != Some(id) {
                    return Cmd::none();
                }
                
                let task_title = self.edit_title.clone();
                let description = self.edit_description.clone();
                
                // Exit edit mode immediately to prevent double-saves
                self.editing_task = None;
                
                Cmd::new(async move {
                    match update_task(id, Some(task_title), Some(description), None).await {
                        Ok(task) => Msg::TaskSaved(task),
                        Err(e) => Msg::Error(e),
                    }
                })
            }
            Msg::CancelEdit => {
                self.editing_task = None;
                Cmd::none()
            }
            Msg::Error(error) => {
                console::log_1(&format!("Error: {}", error).into());
                Cmd::none()
            }
        }
    }

    fn view(&self) -> Node<Msg> {
        div(
            [class("max-w-4xl mx-auto p-6")],
            [
                div(
                    [class("bg-white rounded-lg shadow-sm p-6")],
                    [
                        h1([class("text-3xl font-bold text-gray-800 text-center mb-8")], [text("Task Manager")]),
                        self.view_create_form(),
                        if self.loading {
                            div([class("text-center py-10 text-gray-500 italic")], [text("Loading...")])
                        } else {
                            self.view_task_list()
                        },
                    ]
                )
            ],
        )
    }
}

impl Model {
    fn view_create_form(&self) -> Node<Msg> {
        div(
            [class("mb-8 p-6 bg-gray-50 rounded-lg")],
            [
                h2([class("text-xl font-semibold text-gray-700 mb-4 pb-2 border-b border-gray-200")], [text("Add New Task")]),
                div([class("space-y-4")], [
                    input([
                        r#type("text"),
                        placeholder("Task title"),
                        value(&self.new_task_title),
                        on_input(|event| Msg::SetNewTaskTitle(event.value())),
                        class("w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"),
                    ], []),
                    textarea([
                        placeholder("Task description"),
                        value(&self.new_task_description),
                        on_input(|event| Msg::SetNewTaskDescription(event.value())),
                        class("w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent h-20 resize-y"),
                    ], []),
                    button([
                        on_click(|_| Msg::CreateTask),
                        class("bg-blue-600 hover:bg-blue-700 text-white font-medium px-6 py-2 rounded-md transition-colors duration-200"),
                    ], [text("Add Task")]),
                ]),
            ],
        )
    }

    fn view_task_list(&self) -> Node<Msg> {
        div(
            [],
            [
                h2([class("text-xl font-semibold text-gray-700 mb-4 pb-2 border-b border-gray-200")], [text("Tasks")]),
                div(
                    [class("space-y-3")],
                    self.tasks.iter().map(|task| self.view_task(task)).collect::<Vec<_>>(),
                ),
            ],
        )
    }

    fn view_task(&self, task: &Task) -> Node<Msg> {
        let is_editing = self.editing_task == Some(task.id);
        let task_id = task.id;
        
        div(
            [class(&format!(
                "border border-gray-200 rounded-lg p-4 bg-white shadow-sm transition-all duration-200 {}",
                if task.completed { "bg-gray-50 opacity-75" } else { "hover:shadow-md" }
            ))],
            if is_editing {
                vec![
                    div([class("space-y-3")], [
                        input([
                            r#type("text"),
                            value(&self.edit_title),
                            on_input(|event| Msg::SetEditTitle(event.value())),
                            class("w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"),
                        ], []),
                        textarea([
                            value(&self.edit_description),
                            on_input(|event| Msg::SetEditDescription(event.value())),
                            class("w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent h-20 resize-y"),
                        ], []),
                        div([class("flex gap-2")], [
                            button([
                                on_click(move |_| Msg::SaveEdit(task_id)),
                                class("bg-green-600 hover:bg-green-700 text-white font-medium px-4 py-2 rounded-md transition-colors duration-200"),
                            ], [text("Save")]),
                            button([
                                on_click(|_| Msg::CancelEdit),
                                class("bg-gray-500 hover:bg-gray-600 text-white font-medium px-4 py-2 rounded-md transition-colors duration-200"),
                            ], [text("Cancel")]),
                        ]),
                    ]),
                ]
            } else {
                vec![
                    div([class("flex items-start justify-between")], [
                        div([class("flex-1")], [
                            h3([class(&format!(
                                "text-lg font-medium mb-2 {}",
                                if task.completed { "line-through text-gray-500" } else { "text-gray-800" }
                            ))], [text(&task.title)]),
                            p([class("text-gray-600 leading-relaxed")], [text(&task.description)]),
                        ]),
                        div([class("flex flex-col gap-2 ml-4")], [
                            div([class("flex items-center")], [
                                input([
                                    r#type("checkbox"),
                                    checked(task.completed),
                                    on_click(move |_| Msg::ToggleTask(task_id)),
                                    class("mr-2 h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"),
                                ], []),
                            ]),
                            div([class("flex gap-1")], [
                                button([
                                    on_click(move |_| Msg::EditTask(task_id)),
                                    class("bg-blue-500 hover:bg-blue-600 text-white text-xs font-medium px-3 py-1 rounded transition-colors duration-200"),
                                ], [text("Edit")]),
                                button([
                                    on_click(move |_| Msg::DeleteTask(task_id)),
                                    class("bg-red-500 hover:bg-red-600 text-white text-xs font-medium px-3 py-1 rounded transition-colors duration-200"),
                                ], [text("Delete")]),
                            ]),
                        ]),
                    ]),
                ]
            },
        )
    }
}

async fn fetch_tasks() -> Result<Vec<Task>, String> {
    let promise = web_sys::window()
        .unwrap()
        .fetch_with_str("/api/tasks");
    
    let response: Response = JsFuture::from(promise)
        .await
        .map_err(|_| "Failed to fetch tasks")?
        .into();

    let text_promise = response.text().map_err(|_| "Failed to read response")?;
    let text = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed to get text")?
        .as_string()
        .ok_or("Failed to convert to string")?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))
}

async fn create_task(task_title: String, description: String) -> Result<Task, String> {
    let request = CreateTaskRequest { title: task_title, description };
    let body = serde_json::to_string(&request).map_err(|_| "Failed to serialize request")?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request = Request::new_with_str_and_init("/api/tasks", &opts)
        .map_err(|_| "Failed to create request")?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|_| "Failed to set header")?;

    let promise = web_sys::window()
        .unwrap()
        .fetch_with_request(&request);

    let response: Response = JsFuture::from(promise)
        .await
        .map_err(|_| "Failed to send request")?
        .into();

    let text_promise = response.text().map_err(|_| "Failed to read response")?;
    let text = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed to get text")?
        .as_string()
        .ok_or("Failed to convert to string")?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))
}

async fn update_task(
    id: Uuid,
    task_title: Option<String>,
    description: Option<String>,
    completed: Option<bool>,
) -> Result<Task, String> {
    let request = UpdateTaskRequest {
        title: task_title,
        description,
        completed,
    };
    let body = serde_json::to_string(&request).map_err(|_| "Failed to serialize request")?;

    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("/api/tasks/{}", id);
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|_| "Failed to create request")?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|_| "Failed to set header")?;

    let promise = web_sys::window()
        .unwrap()
        .fetch_with_request(&request);

    let response: Response = JsFuture::from(promise)
        .await
        .map_err(|_| "Failed to send request")?
        .into();

    let text_promise = response.text().map_err(|_| "Failed to read response")?;
    let text = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed to get text")?
        .as_string()
        .ok_or("Failed to convert to string")?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))
}

async fn delete_task(id: Uuid) -> Result<(), String> {
    let opts = RequestInit::new();
    opts.set_method("DELETE");

    let url = format!("/api/tasks/{}", id);
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|_| "Failed to create request")?;

    let promise = web_sys::window()
        .unwrap()
        .fetch_with_request(&request);

    JsFuture::from(promise)
        .await
        .map_err(|_| "Failed to send request")?;

    Ok(())
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    Program::mount_to_body(Model::default());
}