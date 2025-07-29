use sauron::{
    html::{attributes, attributes::*, *},
    prelude::*,
};
use shared::{CreateTaskRequest, Task, UpdateTaskRequest};
use uuid::Uuid;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, Request, RequestInit, Response};

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Dashboard,
    Tasks,
    Analytics,
    Settings,
}

impl Page {
    fn to_path(&self) -> &'static str {
        match self {
            Page::Dashboard => "/",
            Page::Tasks => "/tasks",
            Page::Analytics => "/analytics", 
            Page::Settings => "/settings",
        }
    }
    
    fn from_path(path: &str) -> Self {
        match path {
            "/" => Page::Dashboard,
            "/tasks" => Page::Tasks,
            "/analytics" => Page::Analytics,
            "/settings" => Page::Settings,
            _ => Page::Dashboard, // Default fallback
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Navigation
    NavigateTo(Page),
    RouteChanged(String),
    
    // Tasks
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
    current_page: Page,
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
            current_page: Page::Dashboard,
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
        // Initialize current page from URL
        if let Some(window) = window() {
            let location = window.location();
            if let Ok(pathname) = location.pathname() {
                self.current_page = Page::from_path(&pathname);
            }
        }
        
        // Set up popstate listener for browser back/forward buttons
        setup_popstate_listener();
        
        // Load tasks for dashboard stats, but don't show loading state
        Cmd::new(async { Msg::LoadTasks })
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::NavigateTo(page) => {
                self.current_page = page.clone();
                
                // Update browser URL without page reload
                if let Some(history) = window().and_then(|w| w.history().ok()) {
                    let _ = history.push_state_with_url(
                        &wasm_bindgen::JsValue::NULL,
                        "",
                        Some(page.to_path())
                    );
                }
                
                // Load tasks when navigating to tasks page
                if self.current_page == Page::Tasks && self.tasks.is_empty() {
                    Cmd::new(async { Msg::LoadTasks })
                } else {
                    Cmd::none()
                }
            }
            Msg::RouteChanged(path) => {
                let new_page = Page::from_path(&path);
                if new_page != self.current_page {
                    self.current_page = new_page;
                    // Load tasks if navigating to tasks page
                    if self.current_page == Page::Tasks && self.tasks.is_empty() {
                        Cmd::new(async { Msg::LoadTasks })
                    } else {
                        Cmd::none()
                    }
                } else {
                    Cmd::none()
                }
            }
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
            [class("min-h-screen bg-gray-100")],
            [
                self.view_header(),
                div(
                    [class("max-w-6xl mx-auto px-6 py-8")],
                    [
                        match self.current_page {
                            Page::Dashboard => self.view_dashboard(),
                            Page::Tasks => self.view_tasks_page(),
                            Page::Analytics => self.view_analytics_page(),
                            Page::Settings => self.view_settings_page(),
                        }
                    ]
                )
            ],
        )
    }
}

impl Model {
    fn view_header(&self) -> Node<Msg> {
        header([class("bg-white shadow-sm border-b border-gray-200")], [
            div([class("max-w-6xl mx-auto px-6 py-4")], [
                div([class("flex items-center justify-between")], [
                    h1([class("text-2xl font-bold text-gray-900")], [text("Full-Stack Rust Demo")]),
                    nav([class("flex space-x-8")], [
                        self.nav_link("Dashboard", Page::Dashboard),
                        self.nav_link("Tasks", Page::Tasks),
                        self.nav_link("Analytics", Page::Analytics),
                        self.nav_link("Settings", Page::Settings),
                    ]),
                ]),
            ]),
        ])
    }

    fn nav_link(&self, label: &str, page: Page) -> Node<Msg> {
        let is_active = self.current_page == page;
        a([
            href(page.to_path()),
            on_click(move |event| {
                event.prevent_default();
                Msg::NavigateTo(page.clone())
            }),
            class(&format!(
                "px-3 py-2 rounded-md text-sm font-medium transition-colors duration-200 {}",
                if is_active {
                    "bg-blue-100 text-blue-700"
                } else {
                    "text-gray-500 hover:text-gray-700 hover:bg-gray-100"
                }
            )),
        ], [text(label)])
    }

    fn view_dashboard(&self) -> Node<Msg> {
        div([class("space-y-8")], [
            // Welcome section
            div([class("bg-white rounded-lg shadow-sm p-8")], [
                h2([class("text-3xl font-bold text-gray-900 mb-4")], [text("Welcome to the Full-Stack Rust Demo")]),
                p([class("text-lg text-gray-600 mb-6")], [text("This application demonstrates a complete full-stack Rust implementation using Axum (backend) and Sauron (frontend) with WebAssembly.")]),
                div([class("grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mt-8")], [
                    self.stat_card("Total Tasks", &self.tasks.len().to_string(), "📝"),
                    self.stat_card("Completed", &self.tasks.iter().filter(|t| t.completed).count().to_string(), "✅"),
                    self.stat_card("Pending", &self.tasks.iter().filter(|t| !t.completed).count().to_string(), "⏳"),
                    self.stat_card("Redis Storage", "Active", "🗄️"),
                ]),
            ]),
            
            // Tech stack section
            div([class("bg-white rounded-lg shadow-sm p-8")], [
                h3([class("text-2xl font-semibold text-gray-900 mb-6")], [text("Technology Stack")]),
                div([class("grid grid-cols-1 md:grid-cols-2 gap-8")], [
                    div([], [
                        h4([class("text-lg font-medium text-gray-900 mb-4")], [text("Backend")]),
                        ul([class("space-y-2 text-gray-600")], [
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-orange-500 rounded-full mr-3")], []), text("Rust + Axum")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-red-500 rounded-full mr-3")], []), text("Redis for persistence")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-blue-500 rounded-full mr-3")], []), text("REST API with CRUD operations")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-green-500 rounded-full mr-3")], []), text("Docker containerization")]),
                        ]),
                    ]),
                    div([], [
                        h4([class("text-lg font-medium text-gray-900 mb-4")], [text("Frontend")]),
                        ul([class("space-y-2 text-gray-600")], [
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-orange-500 rounded-full mr-3")], []), text("Rust + Sauron")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-purple-500 rounded-full mr-3")], []), text("WebAssembly (WASM)")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-cyan-500 rounded-full mr-3")], []), text("Tailwind CSS")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-yellow-500 rounded-full mr-3")], []), text("Reactive UI with Elm architecture")]),
                        ]),
                    ]),
                ]),
            ]),
            
            // Quick actions
            div([class("bg-white rounded-lg shadow-sm p-8")], [
                h3([class("text-2xl font-semibold text-gray-900 mb-6")], [text("Quick Actions")]),
                div([class("flex flex-wrap gap-4")], [
                    a([
                        href(Page::Tasks.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Tasks)
                        }),
                        class("bg-blue-600 hover:bg-blue-700 text-white font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("Manage Tasks")]),
                    a([
                        href(Page::Analytics.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Analytics)
                        }),
                        class("bg-green-600 hover:bg-green-700 text-white font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("View Analytics")]),
                    a([
                        href(Page::Settings.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Settings)
                        }),
                        class("bg-gray-600 hover:bg-gray-700 text-white font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("Settings")]),
                ]),
            ]),
        ])
    }

    fn stat_card(&self, card_title: &str, value: &str, icon: &str) -> Node<Msg> {
        div([class("bg-gray-50 rounded-lg p-6")], [
            div([class("flex items-center justify-between")], [
                div([], [
                    p([class("text-sm font-medium text-gray-600")], [text(card_title)]),
                    p([class("text-2xl font-bold text-gray-900 mt-1")], [text(value)]),
                ]),
                span([class("text-3xl")], [text(icon)]),
            ]),
        ])
    }

    fn view_tasks_page(&self) -> Node<Msg> {
        div([class("bg-white rounded-lg shadow-sm p-6")], [
            h2([class("text-2xl font-bold text-gray-900 mb-6")], [text("Task Management")]),
            self.view_create_form(),
            if self.loading {
                div([class("text-center py-10 text-gray-500 italic")], [text("Loading...")])
            } else {
                self.view_task_list()
            },
        ])
    }

    fn view_analytics_page(&self) -> Node<Msg> {
        div([class("space-y-6")], [
            div([class("bg-white rounded-lg shadow-sm p-8")], [
                h2([class("text-2xl font-bold text-gray-900 mb-6")], [text("Analytics Dashboard")]),
                div([class("grid grid-cols-1 md:grid-cols-3 gap-6 mb-8")], [
                    self.metric_card("Completion Rate", &format!("{}%", if self.tasks.is_empty() { 0 } else { (self.tasks.iter().filter(|t| t.completed).count() * 100) / self.tasks.len() })),
                    self.metric_card("Average Task Length", &format!("{} chars", if self.tasks.is_empty() { 0 } else { self.tasks.iter().map(|t| t.description.len()).sum::<usize>() / self.tasks.len() })),
                    self.metric_card("Most Active Hour", "10:00 AM"),
                ]),
                div([class("bg-gray-50 rounded-lg p-6")], [
                    h3([class("text-lg font-semibold text-gray-900 mb-4")], [text("Task Status Distribution")]),
                    div([class("space-y-3")], [
                        self.progress_bar("Completed", self.tasks.iter().filter(|t| t.completed).count(), self.tasks.len(), "bg-green-500"),
                        self.progress_bar("Pending", self.tasks.iter().filter(|t| !t.completed).count(), self.tasks.len(), "bg-yellow-500"),
                    ]),
                ]),
            ]),
        ])
    }

    fn metric_card(&self, card_title: &str, value: &str) -> Node<Msg> {
        div([class("bg-gray-50 rounded-lg p-6 text-center")], [
            h3([class("text-sm font-medium text-gray-600 mb-2")], [text(card_title)]),
            p([class("text-3xl font-bold text-gray-900")], [text(value)]),
        ])
    }

    fn progress_bar(&self, label: &str, value: usize, total: usize, color_class: &str) -> Node<Msg> {
        let percentage = if total == 0 { 0 } else { (value * 100) / total };
        div([class("flex items-center justify-between")], [
            span([class("text-sm font-medium text-gray-700")], [text(&format!("{} ({})", label, value))]),
            div([class("flex-1 mx-4")], [
                div([class("w-full bg-gray-200 rounded-full h-2")], [
                    div([
                        class(&format!("{} h-2 rounded-full transition-all duration-500", color_class)),
                        attributes::styles([("width", format!("{}%", percentage))]),
                    ], []),
                ]),
            ]),
            span([class("text-sm text-gray-500")], [text(&format!("{}%", percentage))]),
        ])
    }

    fn view_settings_page(&self) -> Node<Msg> {
        div([class("space-y-6")], [
            div([class("bg-white rounded-lg shadow-sm p-8")], [
                h2([class("text-2xl font-bold text-gray-900 mb-6")], [text("Application Settings")]),
                div([class("space-y-6")], [
                    div([], [
                        h3([class("text-lg font-semibold text-gray-900 mb-3")], [text("System Information")]),
                        div([class("bg-gray-50 rounded-lg p-4 space-y-2")], [
                            self.setting_row("Backend", "Axum + Redis"),
                            self.setting_row("Frontend", "Sauron + WebAssembly"),
                            self.setting_row("Database", "Redis (In-memory)"),
                            self.setting_row("Build", "Docker + GitHub Actions"),
                        ]),
                    ]),
                    div([], [
                        h3([class("text-lg font-semibold text-gray-900 mb-3")], [text("API Endpoints")]),
                        div([class("bg-gray-50 rounded-lg p-4 space-y-2 font-mono text-sm")], [
                            self.api_row("GET", "/api/tasks", "List all tasks"),
                            self.api_row("POST", "/api/tasks", "Create new task"),
                            self.api_row("PUT", "/api/tasks/:id", "Update task"),
                            self.api_row("DELETE", "/api/tasks/:id", "Delete task"),
                        ]),
                    ]),
                    div([], [
                        h3([class("text-lg font-semibold text-gray-900 mb-3")], [text("Performance")]),
                        div([class("bg-gray-50 rounded-lg p-4")], [
                            p([class("text-gray-600")], [text("WebAssembly provides near-native performance in the browser, while Rust's zero-cost abstractions ensure efficient backend operations.")]),
                        ]),
                    ]),
                ]),
            ]),
        ])
    }

    fn setting_row(&self, key: &str, value: &str) -> Node<Msg> {
        div([class("flex justify-between items-center")], [
            span([class("text-gray-700 font-medium")], [text(key)]),
            span([class("text-gray-600")], [text(value)]),
        ])
    }

    fn api_row(&self, method: &str, endpoint: &str, description: &str) -> Node<Msg> {
        div([class("flex items-center space-x-4")], [
            span([class(&format!("px-2 py-1 rounded text-xs font-medium text-white {}", 
                match method {
                    "GET" => "bg-green-500",
                    "POST" => "bg-blue-500", 
                    "PUT" => "bg-yellow-500",
                    "DELETE" => "bg-red-500",
                    _ => "bg-gray-500"
                }
            ))], [text(method)]),
            span([class("text-gray-900 font-medium")], [text(endpoint)]),
            span([class("text-gray-600")], [text(description)]),
        ])
    }
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
                if self.tasks.is_empty() {
                    div([class("text-center py-12")], [
                        div([class("text-gray-400 text-6xl mb-4")], [text("📝")]),
                        h3([class("text-lg font-medium text-gray-900 mb-2")], [text("No tasks yet")]),
                        p([class("text-gray-500")], [text("Create your first task above to get started!")]),
                    ])
                } else {
                    div(
                        [class("space-y-4")],
                        self.tasks.iter().map(|task| self.view_task(task)).collect::<Vec<_>>(),
                    )
                },
            ],
        )
    }

    fn view_task(&self, task: &Task) -> Node<Msg> {
        let is_editing = self.editing_task == Some(task.id);
        let task_id = task.id;
        
        div(
            [class(&format!(
                "group border rounded-xl p-6 bg-white shadow-sm transition-all duration-300 hover:shadow-lg {}",
                if task.completed { 
                    "border-green-200 bg-green-50/30" 
                } else { 
                    "border-gray-200 hover:border-blue-200 hover:-translate-y-0.5" 
                }
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
                    div([class("flex items-start gap-4")], [
                        // Enhanced checkbox with visual feedback
                        div([class("flex-shrink-0 pt-1")], [
                            label([class("relative flex items-center cursor-pointer")], [
                                input([
                                    r#type("checkbox"),
                                    checked(task.completed),
                                    on_click(move |_| Msg::ToggleTask(task_id)),
                                    class("sr-only"),
                                ], []),
                                div([class(&format!(
                                    "w-6 h-6 rounded-lg border-2 flex items-center justify-center transition-all duration-200 {}",
                                    if task.completed {
                                        "bg-green-500 border-green-500 shadow-sm"
                                    } else {
                                        "border-gray-300 hover:border-blue-400 hover:bg-blue-50"
                                    }
                                ))], [
                                    if task.completed {
                                        span([class("text-white text-sm font-bold")], [text("✓")])
                                    } else {
                                        span([], [])
                                    }
                                ]),
                            ]),
                        ]),
                        
                        // Task content with improved layout
                        div([class("flex-1 min-w-0")], [
                            h3([class(&format!(
                                "text-lg font-semibold mb-2 transition-all duration-200 {}",
                                if task.completed { 
                                    "line-through text-gray-400" 
                                } else { 
                                    "text-gray-900" 
                                }
                            ))], [text(&task.title)]),
                            p([class(&format!(
                                "text-sm leading-relaxed break-words {}",
                                if task.completed { 
                                    "text-gray-400 line-through" 
                                } else { 
                                    "text-gray-600" 
                                }
                            ))], [text(&task.description)]),
                            
                            // Completion status badge
                            if task.completed {
                                div([class("mt-3")], [
                                    span([class("inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800")], [
                                        span([class("w-1.5 h-1.5 bg-green-400 rounded-full mr-1.5")], []),
                                        text("Completed")
                                    ])
                                ])
                            } else {
                                div([class("mt-3")], [
                                    span([class("inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800")], [
                                        span([class("w-1.5 h-1.5 bg-yellow-400 rounded-full mr-1.5")], []),
                                        text("Pending")
                                    ])
                                ])
                            }
                        ]),
                        
                        // Action buttons with improved styling
                        div([class("flex-shrink-0")], [
                            div([class("flex flex-col gap-2")], [
                                button([
                                    on_click(move |_| Msg::EditTask(task_id)),
                                    class("inline-flex items-center justify-center w-8 h-8 rounded-lg bg-blue-100 text-blue-600 hover:bg-blue-200 transition-colors duration-200 group"),
                                    r#type("button"),
                                ], [
                                    span([class("text-sm")], [text("✏️")])
                                ]),
                                button([
                                    on_click(move |_| Msg::DeleteTask(task_id)),
                                    class("inline-flex items-center justify-center w-8 h-8 rounded-lg bg-red-100 text-red-600 hover:bg-red-200 transition-colors duration-200 group"),
                                    r#type("button"),
                                ], [
                                    span([class("text-sm")], [text("🗑️")])
                                ]),
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

fn setup_popstate_listener() {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    
    let callback = Closure::wrap(Box::new(|_event: web_sys::PopStateEvent| {
        if let Some(window) = window() {
            let location = window.location();
            if let Ok(pathname) = location.pathname() {
                // This would need to be connected to the application's message system
                // For now, we'll just log it
                console::log_1(&format!("Route changed to: {}", pathname).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    window()
        .unwrap()
        .add_event_listener_with_callback("popstate", callback.as_ref().unchecked_ref())
        .unwrap();
    
    callback.forget();
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    Program::mount_to_body(Model::default());
}