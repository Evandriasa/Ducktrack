import { call, loadTasks, loadProjects } from "./api.js";
import { esc, STATUSES, dot, statusBadge, prioClass, fmtDur, todayLocal } from "./ui.js";

let currentFilter = { include_subtasks: false, sort: "created_new" };

const PRIO_ORDER = ["Critical", "High", "Medium", "Low"];

function key(t) { return t.task || t; }

function sortTasks(list) {
  const arr = [...(list || [])];
  const s = currentFilter.sort || "created_new";
  arr.sort((a, b) => {
    const ka = key(a), kb = key(b);
    switch (s) {
      case "created_old": return (ka.createdAt || "").localeCompare(kb.createdAt || "");
      case "due": return (ka.dueDate || "9999").localeCompare(kb.dueDate || "9999");
      case "priority":
        return (PRIO_ORDER.indexOf(ka.priority) - PRIO_ORDER.indexOf(kb.priority)) ||
          (kb.createdAt || "").localeCompare(ka.createdAt || "");
      case "title": return (ka.title || "").localeCompare(kb.title || "");
      case "updated": return (kb.updatedAt || "").localeCompare(ka.updatedAt || "");
      case "status":
        return (ka.status || "").localeCompare(kb.status || "") ||
          (kb.createdAt || "").localeCompare(ka.createdAt || "");
      default: return (kb.createdAt || "").localeCompare(ka.createdAt || "");
    }
  });
  return arr;
}

export async function render(container) {
  let tasks = [];
  try { tasks = await loadTasks(currentFilter); } catch {}

  let projects = [];
  try { projects = await loadProjects(); } catch {}

  let tags = [];
  try { tags = await call("list_tags"); } catch {}

  const today = todayLocal();

  const filterBar = `
    <div class="toolbar">
      <input id="task-filter" class="input" placeholder="Filter tasks…" style="max-width:200px">
      <select id="task-status-filter" class="select" style="max-width:140px">
        <option value="">All statuses</option>
        ${STATUSES.map(s => `<option value="${s}" ${currentFilter.status === s ? 'selected' : ''}>${s}</option>`).join("")}
      </select>
      <select id="task-tag-filter" class="select" style="max-width:160px">
        <option value="">All tags</option>
        ${(tags || []).filter(t => t.taskCount > 0).map(t => `<option value="${t.id}" ${currentFilter.tagId === t.id ? 'selected' : ''}>${esc(t.name)}</option>`).join("")}
      </select>
      <select id="task-sort" class="select" style="max-width:160px">
        <option value="created_new" ${currentFilter.sort==='created_new'?'selected':''}>Newest first</option>
        <option value="created_old" ${currentFilter.sort==='created_old'?'selected':''}>Oldest first</option>
        <option value="due" ${currentFilter.sort==='due'?'selected':''}>Due date</option>
        <option value="priority" ${currentFilter.sort==='priority'?'selected':''}>Priority</option>
        <option value="title" ${currentFilter.sort==='title'?'selected':''}>Title A–Z</option>
        <option value="updated" ${currentFilter.sort==='updated'?'selected':''}>Recently updated</option>
      </select>
      <label class="row" style="gap:6px;font-size:12px;color:var(--muted)">
        <input type="checkbox" id="task-subtasks" ${currentFilter.include_subtasks ? 'checked' : ''}> Include subtasks
      </label>
    </div>`;

  const rows = sortTasks(tasks).map(t => {
    const task = key(t);
    const id = task.id;
    const status = task.status || "Todo";
    const overdue = task.dueDate && !["Completed", "Cancelled"].includes(status) && task.dueDate < today;
    return `<div class="row-item" data-id="${id}">
      ${dot(status)}
      <span class="key" style="width:90px">${esc(task.key)}</span>
      <div class="grow"><div class="small">${esc(task.title)}</div><div class="xs muted mt-1">${esc(t.projectName || t.projectKey || '')}</div></div>
      ${task.estimatedMinutes ? `<span class="xs muted" style="width:60px">~${fmtDur(task.estimatedMinutes)}</span>` : `<span style="width:60px"></span>`}
      <span class="${prioClass(task.priority)}" style="width:70px">${esc(task.priority)}</span>
      <span style="width:96px">${overdue ? '<span class="badge b-overdue" title="Due ' + esc(task.dueDate) + '">overdue</span>' : (task.dueDate ? `<span class="xs muted">${esc(task.dueDate)}</span>` : '')}</span>
      ${statusBadge(status)}
    </div>`;
  }).join("");

  container.innerHTML = `
    <div class="page-max">
      <div class="page-head">
        <div>
          <div class="eyebrow">Work queue</div>
          <h1 class="page-title">My Tasks</h1>
          <p class="page-desc">Everything assigned to you, without the noise.</p>
        </div>
        <button class="btn btn-primary" id="tasks-new">+ New task</button>
      </div>
      <div class="card" style="overflow:hidden">
        ${filterBar}
        <div id="tasks-list">${rows || '<div class="empty"><div class="big">No tasks found</div></div>'}</div>
      </div>
    </div>`;

  document.getElementById("tasks-new")?.addEventListener("click", async () => {
    const { openNewTaskModal } = await import("./app.js");
    openNewTaskModal();
  });

  document.getElementById("task-status-filter")?.addEventListener("change", async (e) => {
    currentFilter.status = e.target.value || undefined;
    await render(container);
  });

  document.getElementById("task-subtasks")?.addEventListener("change", async (e) => {
    currentFilter.include_subtasks = e.target.checked;
    await render(container);
  });

  document.getElementById("task-tag-filter")?.addEventListener("change", async (e) => {
    currentFilter.tagId = e.target.value ? Number(e.target.value) : undefined;
    await render(container);
  });

  document.getElementById("task-sort")?.addEventListener("change", async (e) => {
    currentFilter.sort = e.target.value || "created_new";
    await render(container);
  });

  let filterTimeout;
  document.getElementById("task-filter")?.addEventListener("input", (e) => {
    clearTimeout(filterTimeout);
    const q = e.target.value.toLowerCase();
    filterTimeout = setTimeout(() => {
      const items = document.querySelectorAll("#tasks-list .row-item");
      items.forEach(item => {
        const text = item.textContent.toLowerCase();
        item.style.display = text.includes(q) ? "" : "none";
      });
    }, 150);
  });

  document.querySelectorAll("#tasks-list .row-item").forEach(el => {
    el.addEventListener("click", async () => {
      const id = Number(el.dataset.id);
      const { openTaskDetail } = await import("./app.js");
      openTaskDetail(id);
    });
  });
}