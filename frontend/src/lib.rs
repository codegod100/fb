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
    RevertTaskToggle(Uuid, bool),
    DeleteTask(Uuid),
    TaskDeleted(Uuid),
    EditTask(Uuid),
    SetEditTitle(String),
    SetEditDescription(String),
    SaveEdit(Uuid),
    TaskSaved(Task),
    CancelEdit,
    ClearCompleted,
    ToggleCompletedSection,
    // Task loading states
    SetTaskLoading(Uuid, bool),
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
    show_completed: bool,
    task_loading_states: std::collections::HashMap<Uuid, bool>, // Track loading state for individual tasks
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
            show_completed: true,
            task_loading_states: std::collections::HashMap::new(),
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
                console::log_1(&format!("[DEBUG] TasksLoaded - {} tasks received", tasks.len()).into());
                for (i, task) in tasks.iter().enumerate() {
                    console::log_1(&format!("[DEBUG] Task {}: ID={}, Title='{}', Completed={}", 
                        i, task.id, task.title, task.completed).into());
                }
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
                console::log_1(&format!("[DEBUG] ToggleTask called for ID: {}", id).into());
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    let old_completed = task.completed;
                    let new_completed = !task.completed;
                    console::log_1(&format!("[DEBUG] Task found - Title: '{}', Old completed: {}, New completed: {}", 
                        task.title, old_completed, new_completed).into());
                    
                    // Set task as loading
                    self.task_loading_states.insert(id, true);
                    
                    // OPTIMISTIC UPDATE: Update local state immediately for responsive UI
                    task.completed = new_completed;
                    console::log_1(&format!("[DEBUG] Optimistic update applied locally").into());
                    
                    // Then sync with server in background
                    Cmd::new(async move {
                        console::log_1(&format!("[DEBUG] Sending background sync request for task {}", id).into());
                        match update_task(id, None, None, Some(new_completed)).await {
                            Ok(updated_task) => {
                                console::log_1(&format!("[DEBUG] Background sync successful - Task: '{}', Completed: {}", 
                                    updated_task.title, updated_task.completed).into());
                                // We could add a message to handle server-client sync conflicts if needed
                                Msg::TaskUpdated(updated_task)
                            },
                            Err(e) => {
                                console::log_1(&format!("[DEBUG] Background sync failed: {}, reverting optimistic update", e).into());
                                // On error, revert the optimistic update
                                Msg::RevertTaskToggle(id, old_completed)
                            },
                        }
                    })
                } else {
                    console::log_1(&format!("[DEBUG] Task with ID {} not found in local state!", id).into());
                    Cmd::none()
                }
            }
            Msg::TaskUpdated(updated_task) => {
                console::log_1(&format!("[DEBUG] TaskUpdated received - ID: {}, Title: '{}', Completed: {}", 
                    updated_task.id, updated_task.title, updated_task.completed).into());
                
                // Remove loading state for this task
                self.task_loading_states.remove(&updated_task.id);
                
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == updated_task.id) {
                    // Only update if the server response differs from our current state
                    // This prevents race conditions where stale responses overwrite newer state
                    if task.completed != updated_task.completed {
                        console::log_1(&format!("[DEBUG] Updating task state from server response").into());
                        *task = updated_task;
                    } else {
                        console::log_1(&format!("[DEBUG] Server response matches current state, no update needed").into());
                    }
                } else {
                    console::log_1(&format!("[DEBUG] WARNING: Could not find task {} in local state to update!", updated_task.id).into());
                }
                Cmd::none()
            }
            Msg::RevertTaskToggle(id, original_completed) => {
                console::log_1(&format!("[DEBUG] Reverting optimistic update for task {} to completed: {}", id, original_completed).into());
                // Remove loading state for this task
                self.task_loading_states.remove(&id);
                
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.completed = original_completed;
                    console::log_1(&format!("[DEBUG] Optimistic update reverted successfully").into());
                } else {
                    console::log_1(&format!("[DEBUG] WARNING: Could not find task {} to revert!", id).into());
                }
                Cmd::none()
            }
            Msg::TaskSaved(saved_task) => {
                // Remove loading state for this task
                self.task_loading_states.remove(&saved_task.id);
                
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
                if window().unwrap().confirm_with_message("Are you sure you want to delete this task?").unwrap() {
                    self.task_loading_states.insert(id, true);
                    Cmd::new(async move {
                        match delete_task(id).await {
                            Ok(_) => Msg::TaskDeleted(id),
                            Err(e) => Msg::Error(e),
                        }
                    })
                } else {
                    Cmd::none()
                }
            }
            Msg::TaskDeleted(id) => {
                self.tasks.retain(|t| t.id != id);
                // Remove loading state for this task
                self.task_loading_states.remove(&id);
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
                
                // Set task as loading
                self.task_loading_states.insert(id, true);
                
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
            Msg::ClearCompleted => {
                if window().unwrap().confirm_with_message("Are you sure you want to clear all completed tasks?").unwrap() {
                    let completed_ids: Vec<Uuid> = self.tasks.iter()
                        .filter(|t| t.completed)
                        .map(|t| t.id)
                        .collect();
                    
                    for id in &completed_ids {
                        self.task_loading_states.insert(*id, true);
                    }
                    
                    self.tasks.retain(|t| !t.completed);
                    
                    Cmd::batch(
                        completed_ids.into_iter()
                            .map(|id| Cmd::new(async move {
                                match delete_task(id).await {
                                    Ok(_) => Msg::TaskDeleted(id),
                                    Err(e) => Msg::Error(e),
                                }
                            }))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    Cmd::none()
                }
            }
            Msg::ToggleCompletedSection => {
                self.show_completed = !self.show_completed;
                Cmd::none()
            }
            Msg::SetTaskLoading(id, loading) => {
                if loading {
                    self.task_loading_states.insert(id, true);
                } else {
                    self.task_loading_states.remove(&id);
                }
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
            [class("min-h-screen bg-ctp-base text-ctp-text")],
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
        header([class("bg-ctp-mantle shadow-lg border-b border-ctp-surface0")], [
            div([class("max-w-6xl mx-auto px-6 py-4")], [
                div([class("flex items-center justify-between")], [
                    h1([class("text-2xl font-bold text-ctp-text")], [text("Full-Stack Rust Demo")]),
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
                    "bg-ctp-blue text-ctp-base"
                } else {
                    "text-ctp-subtext0 hover:text-ctp-text hover:bg-ctp-surface0"
                }
            )),
        ], [text(label)])
    }

    fn view_dashboard(&self) -> Node<Msg> {
        div([class("space-y-8")], [
            // Welcome section
            div([class("bg-ctp-surface0 rounded-lg shadow-lg p-8 border border-ctp-surface1")], [
                h2([class("text-3xl font-bold text-ctp-text mb-4")], [text("Welcome to the Full-Stack Rust Demo")]),
                p([class("text-lg text-ctp-subtext1 mb-6")], [text("This application demonstrates a complete full-stack Rust implementation using Axum (backend) and Sauron (frontend) with WebAssembly.")]),
                div([class("grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mt-8")], [
                    self.stat_card("Total Tasks", &self.tasks.len().to_string(), "📝"),
                    self.stat_card("Completed", &self.tasks.iter().filter(|t| t.completed).count().to_string(), "✅"),
                    self.stat_card("Pending", &self.tasks.iter().filter(|t| !t.completed).count().to_string(), "⏳"),
                    self.stat_card("Redis Storage", "Active", "🗄️"),
                ]),
            ]),
            
            // Tech stack section
            div([class("bg-ctp-surface0 rounded-lg shadow-lg p-8 border border-ctp-surface1")], [
                h3([class("text-2xl font-semibold text-ctp-text mb-6")], [text("Technology Stack")]),
                div([class("grid grid-cols-1 md:grid-cols-2 gap-8")], [
                    div([], [
                        h4([class("text-lg font-medium text-ctp-text mb-4")], [text("Backend")]),
                        ul([class("space-y-2 text-ctp-subtext1")], [
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-peach rounded-full mr-3")], []), text("Rust + Axum")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-red rounded-full mr-3")], []), text("Redis for persistence")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-blue rounded-full mr-3")], []), text("REST API with CRUD operations")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-green rounded-full mr-3")], []), text("Docker containerization")]),
                        ]),
                    ]),
                    div([], [
                        h4([class("text-lg font-medium text-ctp-text mb-4")], [text("Frontend")]),
                        ul([class("space-y-2 text-ctp-subtext1")], [
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-peach rounded-full mr-3")], []), text("Rust + Sauron")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-mauve rounded-full mr-3")], []), text("WebAssembly (WASM)")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-teal rounded-full mr-3")], []), text("Tailwind CSS")]),
                            li([class("flex items-center")], [span([class("w-2 h-2 bg-ctp-yellow rounded-full mr-3")], []), text("Reactive UI with Elm architecture")]),
                        ]),
                    ]),
                ]),
            ]),
            
            // Quick actions
            div([class("bg-ctp-surface0 rounded-lg shadow-lg p-8 border border-ctp-surface1")], [
                h3([class("text-2xl font-semibold text-ctp-text mb-6")], [text("Quick Actions")]),
                div([class("flex flex-wrap gap-4")], [
                    a([
                        href(Page::Tasks.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Tasks)
                        }),
                        class("bg-ctp-blue hover:bg-ctp-sapphire text-ctp-base font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("Manage Tasks")]),
                    a([
                        href(Page::Analytics.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Analytics)
                        }),
                        class("bg-ctp-green hover:bg-ctp-teal text-ctp-base font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("View Analytics")]),
                    a([
                        href(Page::Settings.to_path()),
                        on_click(|event| {
                            event.prevent_default();
                            Msg::NavigateTo(Page::Settings)
                        }),
                        class("bg-ctp-mauve hover:bg-ctp-lavender text-ctp-base font-medium px-6 py-3 rounded-lg transition-colors duration-200 inline-block"),
                    ], [text("Settings")]),
                ]),
            ]),
        ])
    }

    fn stat_card(&self, card_title: &str, value: &str, icon: &str) -> Node<Msg> {
        div([class("bg-ctp-surface1 rounded-lg p-6 border border-ctp-surface2")], [
            div([class("flex items-center justify-between")], [
                div([], [
                    p([class("text-sm font-medium text-ctp-subtext0")], [text(card_title)]),
                    p([class("text-2xl font-bold text-ctp-text mt-1")], [text(value)]),
                ]),
                span([class("text-3xl")], [text(icon)]),
            ]),
        ])
    }

    fn view_tasks_page(&self) -> Node<Msg> {
        div([class("bg-ctp-surface0 rounded-lg shadow-lg p-6 border border-ctp-surface1")], [
            h2([class("text-2xl font-bold text-ctp-text mb-6")], [text("Task Management")]),
            self.view_create_form(),
            if self.loading {
                div([class("text-center py-10 text-ctp-subtext0 italic")], [text("Loading...")])
            } else {
                self.view_task_list()
            },
        ])
    }

    fn view_analytics_page(&self) -> Node<Msg> {
        div([class("space-y-6")], [
            div([class("bg-ctp-surface0 rounded-lg shadow-lg p-8 border border-ctp-surface1")], [
                h2([class("text-2xl font-bold text-ctp-text mb-6")], [text("Analytics Dashboard")]),
                div([class("grid grid-cols-1 md:grid-cols-3 gap-6 mb-8")], [
                    self.metric_card("Completion Rate", &format!("{}%", if self.tasks.is_empty() { 0 } else { (self.tasks.iter().filter(|t| t.completed).count() * 100) / self.tasks.len() })),
                    self.metric_card("Average Task Length", &format!("{} chars", if self.tasks.is_empty() { 0 } else { self.tasks.iter().map(|t| t.description.len()).sum::<usize>() / self.tasks.len() })),
                    self.metric_card("Most Active Hour", "10:00 AM"),
                ]),
                div([class("bg-ctp-surface1 rounded-lg p-6 border border-ctp-surface2")], [
                    h3([class("text-lg font-semibold text-ctp-text mb-4")], [text("Task Status Distribution")]),
                    div([class("space-y-3")], [
                        self.progress_bar("Completed", self.tasks.iter().filter(|t| t.completed).count(), self.tasks.len(), "bg-ctp-green"),
                        self.progress_bar("Pending", self.tasks.iter().filter(|t| !t.completed).count(), self.tasks.len(), "bg-ctp-yellow"),
                    ]),
                ]),
            ]),
        ])
    }

    fn metric_card(&self, card_title: &str, value: &str) -> Node<Msg> {
        div([class("bg-ctp-surface1 rounded-lg p-6 text-center border border-ctp-surface2")], [
            h3([class("text-sm font-medium text-ctp-subtext0 mb-2")], [text(card_title)]),
            p([class("text-3xl font-bold text-ctp-text")], [text(value)]),
        ])
    }

    fn progress_bar(&self, label: &str, value: usize, total: usize, color_class: &str) -> Node<Msg> {
        let percentage = if total == 0 { 0 } else { (value * 100) / total };
        div([class("flex items-center justify-between")], [
            span([class("text-sm font-medium text-ctp-text")], [text(&format!("{} ({})", label, value))]),
            div([class("flex-1 mx-4")], [
                div([class("w-full bg-ctp-surface2 rounded-full h-2")], [
                    div([
                        class(&format!("{} h-2 rounded-full transition-all duration-500", color_class)),
                        attributes::styles([("width", format!("{}%", percentage))]),
                    ], []),
                ]),
            ]),
            span([class("text-sm text-ctp-subtext0")], [text(&format!("{}%", percentage))]),
        ])
    }

    fn view_settings_page(&self) -> Node<Msg> {
        div([class("space-y-6")], [
            div([class("bg-ctp-surface0 rounded-lg shadow-lg p-8 border border-ctp-surface1")], [
                h2([class("text-2xl font-bold text-ctp-text mb-6")], [text("Application Settings")]),
                div([class("space-y-6")], [
                    div([], [
                        h3([class("text-lg font-semibold text-ctp-text mb-3")], [text("System Information")]),
                        div([class("bg-ctp-surface1 rounded-lg p-4 space-y-2 border border-ctp-surface2")], [
                            self.setting_row("Backend", "Axum + Redis"),
                            self.setting_row("Frontend", "Sauron + WebAssembly"),
                            self.setting_row("Database", "Redis (In-memory)"),
                            self.setting_row("Build", "Docker + GitHub Actions"),
                        ]),
                    ]),
                    div([], [
                        h3([class("text-lg font-semibold text-ctp-text mb-3")], [text("API Endpoints")]),
                        div([class("bg-ctp-surface1 rounded-lg p-4 space-y-2 font-mono text-sm border border-ctp-surface2")], [
                            self.api_row("GET", "/api/tasks", "List all tasks"),
                            self.api_row("POST", "/api/tasks", "Create new task"),
                            self.api_row("PUT", "/api/tasks/:id", "Update task"),
                            self.api_row("DELETE", "/api/tasks/:id", "Delete task"),
                        ]),
                    ]),
                    div([], [
                        h3([class("text-lg font-semibold text-ctp-text mb-3")], [text("Performance")]),
                        div([class("bg-ctp-surface1 rounded-lg p-4 border border-ctp-surface2")], [
                            p([class("text-ctp-subtext1")], [text("WebAssembly provides near-native performance in the browser, while Rust's zero-cost abstractions ensure efficient backend operations.")]),
                        ]),
                    ]),
                ]),
            ]),
        ])
    }

    fn setting_row(&self, key: &str, value: &str) -> Node<Msg> {
        div([class("flex justify-between items-center")], [
            span([class("text-ctp-text font-medium")], [text(key)]),
            span([class("text-ctp-subtext1")], [text(value)]),
        ])
    }

    fn api_row(&self, method: &str, endpoint: &str, description: &str) -> Node<Msg> {
        div([class("flex items-center space-x-4")], [
            span([class(&format!("px-2 py-1 rounded text-xs font-medium text-ctp-base {}", 
                match method {
                    "GET" => "bg-ctp-green",
                    "POST" => "bg-ctp-blue", 
                    "PUT" => "bg-ctp-yellow",
                    "DELETE" => "bg-ctp-red",
                    _ => "bg-ctp-overlay0"
                }
            ))], [text(method)]),
            span([class("text-ctp-text font-medium")], [text(endpoint)]),
            span([class("text-ctp-subtext1")], [text(description)]),
        ])
    }
    fn view_create_form(&self) -> Node<Msg> {
        div(
            [class("mb-8 p-6 bg-ctp-surface1 rounded-lg border border-ctp-surface2")],
            [
                h2([class("text-xl font-semibold text-ctp-text mb-4 pb-2 border-b border-ctp-surface2")], [text("Add New Task")]),
                div([class("space-y-4")], [
                    input([
                        r#type("text"),
                        placeholder("Task title"),
                        value(&self.new_task_title),
                        on_input(|event| Msg::SetNewTaskTitle(event.value())),
                        class("w-full px-3 py-2 bg-ctp-surface0 border border-ctp-surface2 rounded-md text-ctp-text placeholder-ctp-subtext0 focus:outline-none focus:ring-2 focus:ring-ctp-blue focus:border-transparent"),
                    ], []),
                    textarea([
                        placeholder("Task description"),
                        value(&self.new_task_description),
                        on_input(|event| Msg::SetNewTaskDescription(event.value())),
                        class("w-full px-3 py-2 bg-ctp-surface0 border border-ctp-surface2 rounded-md text-ctp-text placeholder-ctp-subtext0 focus:outline-none focus:ring-2 focus:ring-ctp-blue focus:border-transparent h-20 resize-y"),
                    ], []),
                    button([
                        on_click(|_| Msg::CreateTask),
                        class("bg-ctp-blue hover:bg-ctp-sapphire text-ctp-base font-medium px-6 py-2 rounded-md transition-colors duration-200"),
                    ], [text("Add Task")]),
                ]),
            ],
        )
    }

    fn view_task_list(&self) -> Node<Msg> {
        let pending_tasks: Vec<&Task> = self.tasks.iter().filter(|t| !t.completed).collect();
        let completed_tasks: Vec<&Task> = self.tasks.iter().filter(|t| t.completed).collect();
        
        console::log_1(&format!("[DEBUG] Task list - Total: {}, Pending: {}, Completed: {}", 
            self.tasks.len(), pending_tasks.len(), completed_tasks.len()).into());
        
        console::log_1(&format!("[DEBUG] Pending tasks: {:?}", 
            pending_tasks.iter().map(|t| format!("{}:{}", t.id, t.title)).collect::<Vec<_>>()).into());
        console::log_1(&format!("[DEBUG] Completed tasks: {:?}", 
            completed_tasks.iter().map(|t| format!("{}:{}", t.id, t.title)).collect::<Vec<_>>()).into());
        
        div(
            [class("space-y-8")],
            [
                // Pending Tasks Section
                div([], [
                    div([class("flex items-center justify-between mb-4")], [
                        h2([class("text-xl font-semibold text-ctp-text pb-2 border-b border-ctp-surface2")], [text("Active Tasks")]),
                        if !pending_tasks.is_empty() {
                            span([class("bg-ctp-blue/20 text-ctp-blue px-2 py-1 rounded-full text-sm font-medium")], [
                                text(&format!("{} active", pending_tasks.len()))
                            ])
                        } else {
                            span([], [])
                        }
                    ]),
                    if pending_tasks.is_empty() {
                        div([class("text-center py-12")], [
                            div([class("text-ctp-overlay0 text-6xl mb-4")], [text("✨")]),
                            h3([class("text-lg font-medium text-ctp-text mb-2")], [text("All caught up!")]),
                            p([class("text-ctp-subtext0")], [text("No active tasks. Create a new one above to get started!")]),
                        ])
                    } else {
                        div(
                            [class("space-y-4")],
                            pending_tasks.iter().map(|task| self.view_task(task)).collect::<Vec<_>>(),
                        )
                    }
                ]),
                
                // Completed Tasks Section
                if !completed_tasks.is_empty() {
                    div([class("border-t border-ctp-surface1 pt-8")], [
                        div([class("flex items-center justify-between mb-4")], [
                            button([
                                on_click(|_| Msg::ToggleCompletedSection),
                                class("flex items-center space-x-2 text-xl font-semibold text-ctp-text hover:text-ctp-blue transition-colors duration-200"),
                            ], [
                                span([], [text("Completed Tasks")]),
                                span([class("text-sm")], [
                                    if self.show_completed {
                                        text("▼")
                                    } else {
                                        text("▶")
                                    }
                                ])
                            ]),
                            div([class("flex items-center space-x-3")], [
                                span([class("bg-ctp-green/20 text-ctp-green px-2 py-1 rounded-full text-sm font-medium")], [
                                    text(&format!("{} completed", completed_tasks.len()))
                                ]),
                                button([
                                    on_click(|_| Msg::ClearCompleted),
                                    class("bg-ctp-red/20 text-ctp-red hover:bg-ctp-red/30 px-3 py-1 rounded-full text-sm font-medium transition-colors duration-200"),
                                ], [text("Clear All")])
                            ])
                        ]),
                        if self.show_completed {
                            div([class("bg-ctp-surface1/50 rounded-lg p-4 border border-ctp-surface2")], [
                                div(
                                    [class("space-y-3")],
                                    completed_tasks.iter().map(|task| self.view_task(task)).collect::<Vec<_>>(),
                                )
                            ])
                        } else {
                            span([], [])
                        }
                    ])
                } else {
                    span([], [])
                }
            ],
        )
    }

    fn view_task(&self, task: &Task) -> Node<Msg> {
        let is_editing = self.editing_task == Some(task.id);
        let is_loading = self.task_loading_states.contains_key(&task.id);
        
        // Debug logging for task rendering
        console::log_1(&format!("[DEBUG] Rendering task - ID: {}, Title: '{}', Completed: {}, Is Editing: {}, Is Loading: {}", 
            task.id, task.title, task.completed, is_editing, is_loading).into());
        
        div(
            [key(task.id.to_string()),
            class(&format!(
                "group border rounded-xl p-6 bg-ctp-surface0 shadow-sm transition-all duration-300 hover:shadow-lg {}",
                if task.completed { 
                    "border-ctp-green bg-ctp-green/10" 
                } else { 
                    "border-ctp-surface1 hover:border-ctp-blue hover:-translate-y-0.5" 
                }
            ))],
            if is_editing {
                vec![
                    div([class("space-y-3")], [
                        input([
                            r#type("text"),
                            value(&self.edit_title),
                            on_input(|event| Msg::SetEditTitle(event.value())),
                            class("w-full px-3 py-2 bg-ctp-surface1 border border-ctp-surface2 rounded-md text-ctp-text focus:outline-none focus:ring-2 focus:ring-ctp-blue focus:border-transparent"),
                        ], []),
                        textarea([
                            value(&self.edit_description),
                            on_input(|event| Msg::SetEditDescription(event.value())),
                            class("w-full px-3 py-2 bg-ctp-surface1 border border-ctp-surface2 rounded-md text-ctp-text focus:outline-none focus:ring-2 focus:ring-ctp-blue focus:border-transparent h-20 resize-y"),
                        ], []),
                        div([class("flex gap-2")], [
                            button([
                                on_click({
                                    let captured_id = task.id;
                                    move |_| Msg::SaveEdit(captured_id)
                                }),
                                class("bg-ctp-green hover:bg-ctp-teal text-ctp-base font-medium px-4 py-2 rounded-md transition-colors duration-200"),
                                disabled(is_loading),
                            ], [
                                if is_loading {
                                    text("Saving...")
                                } else {
                                    text("Save")
                                }
                            ]),
                            button([
                                on_click(|_| Msg::CancelEdit),
                                class("bg-ctp-overlay0 hover:bg-ctp-overlay1 text-ctp-text font-medium px-4 py-2 rounded-md transition-colors duration-200"),
                                disabled(is_loading),
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
                                    id(&format!("checkbox-{}", task.id)), // Add unique ID
                                    on_click({
                                        let task_id = task.id;
                                        move |_| Msg::ToggleTask(task_id)
                                    }),
                                    class("sr-only"),
                                    disabled(is_loading),
                                ], []),
                                div([class(&format!(
                                    "w-6 h-6 rounded-lg border-2 flex items-center justify-center transition-all duration-200 {}",
                                    if task.completed {
                                        "bg-ctp-green border-ctp-green shadow-sm"
                                    } else {
                                        "border-ctp-surface2 hover:border-ctp-blue hover:bg-ctp-blue/10"
                                    }
                                ))], [
                                    if task.completed {
                                        span([class("text-ctp-base text-sm font-bold")], [text("✓")])
                                    } else if is_loading {
                                        // Show loading spinner when task is being updated
                                        span([class("animate-spin text-ctp-blue")], [text("◐")])
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
                                    "line-through text-ctp-overlay1" 
                                } else { 
                                    "text-ctp-text" 
                                }
                            ))], [
                                if is_loading {
                                    text(&format!("{} (updating...)", task.title))
                                } else {
                                    text(&task.title)
                                }
                            ]),
                            p([class(&format!(
                                "text-sm leading-relaxed break-words {}",
                                if task.completed { 
                                    "text-ctp-overlay0 line-through" 
                                } else { 
                                    "text-ctp-subtext1" 
                                }
                            ))], [text(&task.description)]),
                            
                            // Completion status badge
                            if task.completed {
                                div([class("mt-3")], [
                                    span([class("inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-ctp-green/20 text-ctp-green")], [
                                        span([class("w-1.5 h-1.5 bg-ctp-green rounded-full mr-1.5")], []),
                                        if is_loading {
                                            text("Updating...")
                                        } else {
                                            text("Completed")
                                        }
                                    ])
                                ])
                            } else {
                                div([class("mt-3")], [
                                    span([class("inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-ctp-yellow/20 text-ctp-yellow")], [
                                        span([class("w-1.5 h-1.5 bg-ctp-yellow rounded-full mr-1.5")], []),
                                        if is_loading {
                                            text("Updating...")
                                        } else {
                                            text("Pending")
                                        }
                                    ])
                                ])
                            }
                        ]),
                        
                        // Action buttons with improved styling
                        div([class("flex-shrink-0")], [
                            div([class("flex flex-col gap-2")], [
                                if !task.completed {
                                    button([
                                        on_click({
                                            let captured_id = task.id;
                                            move |_| Msg::EditTask(captured_id)
                                        }),
                                        class("inline-flex items-center justify-center w-8 h-8 rounded-lg bg-ctp-blue/20 text-ctp-blue hover:bg-ctp-blue/30 transition-colors duration-200 group"),
                                        r#type("button"),
                                        disabled(is_loading),
                                    ], [
                                        span([class("text-sm")], [text("✏️")])
                                    ])
                                } else {
                                    span([], [])
                                },
                                button([
                                    on_click({
                                        let captured_id = task.id;
                                        move |_| Msg::DeleteTask(captured_id)
                                    }),
                                    class(&format!(
                                        "inline-flex items-center justify-center w-8 h-8 rounded-lg transition-colors duration-200 group {}",
                                        if task.completed {
                                            "bg-ctp-overlay0/20 text-ctp-overlay0 hover:bg-ctp-red/20 hover:text-ctp-red"
                                        } else {
                                            "bg-ctp-red/20 text-ctp-red hover:bg-ctp-red/30"
                                        }
                                    )),
                                    r#type("button"),
                                    disabled(is_loading),
                                ], [
                                    span([class("text-sm")], [
                                        if is_loading {
                                            text("⏳")
                                        } else {
                                            text("🗑️")
                                        }
                                    ])
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
    console::log_1(&format!("[DEBUG] update_task called - ID: {}, completed: {:?}", id, completed).into());
    
    let request = UpdateTaskRequest {
        title: task_title,
        description,
        completed,
    };
    let body = serde_json::to_string(&request).map_err(|_| "Failed to serialize request")?;
    
    console::log_1(&format!("[DEBUG] Update request - Only updating completion status to: {:?} (title/description preserved)", completed).into());
    console::log_1(&format!("[DEBUG] Request body: {}", body).into());

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

    console::log_1(&format!("[DEBUG] Update response text: {}", text).into());
    
    let parsed_task: Task = serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    console::log_1(&format!("[DEBUG] Parsed updated task - ID: {}, Title: '{}', Completed: {}", 
        parsed_task.id, parsed_task.title, parsed_task.completed).into());
    
    Ok(parsed_task)
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