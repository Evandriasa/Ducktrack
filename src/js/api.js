const { invoke } = window.__TAURI__.core;

export async function call(command, args = {}) {
  try {
    return await invoke(command, args);
  } catch (err) {
    const message =
      (err && err.message) ||
      (typeof err === "string" ? err : "Unknown backend error");
    throw new Error(message);
  }
}

export async function loadProjects() {
  return call("list_projects");
}

export async function loadProjectTree() {
  return call("get_project_tree");
}

export async function loadTasks(filter = {}) {
  return call("list_tasks", { filter });
}