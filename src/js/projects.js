import { call, loadProjects } from "./api.js";
import { esc, STATUSES, dot, statusBadge } from "./ui.js";

function treeHtml(nodes, depth = 0) {
  return nodes.map(node => {
    const children = node.children.length ? treeHtml(node.children, depth + 1) : "";
    return `<li>
      <div class="tree-node" data-id="${node.id}">
        <span class="muted">${node.children.length ? "▾" : "·"}</span>
        <span class="key">${esc(node.key)}</span>
        <span class="small">${esc(node.name)}</span>
      </div>
      ${children ? `<ul>${children}</ul>` : ""}
    </li>`;
  }).join("");
}

let viewMode = "cards";

export async function render(container) {
  let tree = [];
  let allProjects = [];
  let tasks = [];
  try {
    [tree, allProjects] = await Promise.all([
      call("get_project_tree"),
      loadProjects(),
    ]);
    tasks = await call("list_tasks", { filter: { include_subtasks: false } });
  } catch {}

  const cards = (allProjects || []).map(p => {
    const pTasks = (tasks || []).filter(t => (t.task ? t.task.projectId : t.projectId) === p.id);
    const total = pTasks.length;
    const done = pTasks.filter(t => (t.task ? t.task.status : t.status) === "Completed").length;
    const pct = total ? Math.round((done / total) * 100) : 0;
    return `<div class="card card-pad" style="cursor:pointer" data-id="${p.id}">
      <div class="spread"><span class="key">${esc(p.key)}</span><span class="dot ${pct===100?'st-done':pct>0?'st-progress':'st-todo'}"></span></div>
      <h3 class="mt-2">${esc(p.name)}</h3>
      <p class="small muted mt-1" style="min-height:38px">${esc(p.description || "")}</p>
      <div class="spread xs muted mt-2 mb-1"><span>Progress</span><span>${pct}%</span></div>
      <div class="bar"><span style="width:${pct}%"></span></div>
      <div class="xs muted mt-2">${total} tasks · ${done} completed</div>
    </div>`;
  }).join("");

  const board = kanbanHtml(tasks);

  const viewTabs = `
    <div class="toolbar">
      <span class="xs muted">View</span>
      <button class="btn btn-sm ${viewMode === 'cards' ? 'btn-primary' : ''}" id="view-cards">Cards</button>
      <button class="btn btn-sm ${viewMode === 'board' ? 'btn-primary' : ''}" id="view-board">Kanban</button>
    </div>`;

  container.innerHTML = `
    <div class="page-max">
      <div class="page-head">
        <div>
          <div class="eyebrow">Workspace structure</div>
          <h1 class="page-title">Projects</h1>
          <p class="page-desc">Projects contain tasks, documentation, changes and files — nested to any depth.</p>
        </div>
        <button class="btn btn-primary" id="projects-new">+ New project</button>
      </div>
      <div class="grid" style="grid-template-columns:2fr 3fr;gap:20px;align-items:start">
        <div class="card card-pad">
          <div class="xs muted mb-2">Project tree</div>
          <div class="tree"><ul>${treeHtml(tree) || '<li class="empty">No projects yet</li>'}</ul></div>
        </div>
        <div>
          <div class="card" style="overflow:hidden;margin-bottom:16px">${viewTabs}</div>
          ${viewMode === 'board'
            ? board || '<div class="card"><div class="empty"><div class="big">No tasks yet</div></div></div>'
            : `<div class="grid" style="grid-template-columns:1fr 1fr;gap:16px">${cards || '<div class="empty">No projects yet</div>'}</div>`}
        </div>
      </div>
    </div>`;

  document.getElementById("view-cards")?.addEventListener("click", async () => {
    viewMode = "cards";
    await render(container);
  });
  document.getElementById("view-board")?.addEventListener("click", async () => {
    viewMode = "board";
    await render(container);
  });

  document.getElementById("projects-new")?.addEventListener("click", async () => {
    const { openNewProjectModal } = await import("./app.js");
    openNewProjectModal();
  });

  document.querySelectorAll(".tree-node, .card[data-id]").forEach(el => {
    el.addEventListener("click", async () => {
      const id = Number(el.dataset.id);
      const { openProjectDetail } = await import("./app.js");
      openProjectDetail(id);
    });
  });

  document.querySelectorAll(".kanban-card").forEach(el => {
    el.addEventListener("click", async () => {
      const id = Number(el.dataset.id);
      const { openTaskDetail } = await import("./app.js");
      openTaskDetail(id);
    });
  });
}

function kanbanHtml(tasks) {
  const grouped = {};
  STATUSES.forEach(s => grouped[s] = []);
  (tasks || []).forEach(t => {
    const status = t.task ? t.task.status : t.status;
    (grouped[status] ||= []).push(t);
  });
  return `<div class="kanban">
    ${STATUSES.map(s => `
      <div class="kanban-col">
        <div class="kanban-head">${dot(s)} ${esc(s)}<span class="xs muted">${grouped[s].length}</span></div>
        ${grouped[s].map(t => {
          const task = t.task || t;
          return `<div class="kanban-card" data-id="${task.id}">
            <div class="spread"><span class="kc-key">${esc(task.key)}</span>${statusBadge(task.status)}</div>
            <div class="kc-title">${esc(task.title)}</div>
            <div class="kc-meta">${esc(t.projectKey || t.projectName || '')}${task.dueDate ? " · due " + esc(task.dueDate) : ""}</div>
          </div>`;
        }).join("") || `<div class="xs muted" style="padding:6px 4px">—</div>`}
      </div>`).join("")}
  </div>`;
}