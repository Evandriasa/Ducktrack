import { call, loadProjects } from "./api.js";
import { esc, fmtDay } from "./ui.js";

export async function render(container) {
  let items = [];
  try { items = await call("list_inbox", { include_completed: true }); } catch (e) {
    container.innerHTML = `<div class="page-max"><div class="empty"><div class="big">Could not load inbox</div><div class="small muted mt-2">${esc(e.message)}</div></div></div>`;
    return;
  }

  const open = items.filter(i => !i.completedAt);
  const done = items.filter(i => !!i.completedAt);

  container.innerHTML = `
    <div class="page-max" style="max-width:980px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Capture</div>
          <h1 class="page-title">Inbox</h1>
          <p class="page-desc">Jot down ideas fast. Promote them into tasks or projects later.</p>
        </div>
      </div>
      <div class="grid" style="grid-template-columns:280px 1fr;gap:20px;align-items:start">
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Quick capture</h3>
          <textarea id="inbox-text" class="textarea" rows="5" placeholder="Anything on your mind…"></textarea>
          <button class="btn btn-primary mt-2 block" id="inbox-add">+ Capture</button>
          <p class="xs muted mt-3">Tip: promote an item to turn it into a real task, or mark it done.</p>
        </div>
        <div class="card" style="overflow:hidden">
          <div class="card-head"><div><h3>Open items</h3><div class="sub">${open.length} remaining</div></div></div>
          <div id="inbox-open">${open.length ? open.map(itemRow).join("") : '<div class="empty"><div class="big">Inbox is clear</div><div class="small muted mt-1">Capture something on the left.</div></div>'}</div>
        </div>
      </div>
      <div class="card mt-3" style="overflow:hidden">
        <div class="card-head"><div><h3>Completed</h3><div class="sub">Promoted or ticked off</div></div></div>
        <div id="inbox-done">${done.length ? done.map(itemRow).join("") : '<div class="empty" style="padding:20px"><div class="big" style="font-size:13px">Nothing completed yet</div></div>'}</div>
      </div>
    </div>`;

  function itemRow(i) {
    const linked = i.promotedTaskId
      ? `<span class="xs muted">→ task ${esc(i.promotedTaskId)}</span>`
      : i.promotedProjectId
        ? `<span class="xs muted">→ project ${esc(i.promotedProjectId)}</span>`
        : "";
    return `<div class="row-item" style="cursor:default">
      <div class="grow">
        <div class="small">${esc(i.text)}${linked}</div>
        <div class="xs muted mt-1">${fmtDay(i.createdAt)}${i.completedAt ? " · done " + fmtDay(i.completedAt) : ""}</div>
      </div>
      ${i.completedAt ? `<button class="btn btn-sm" data-reopen="${i.id}" title="Reopen">↺</button>` : `<button class="btn btn-sm" data-promote="${i.id}">→ Task</button>
      <button class="btn btn-sm" data-done="${i.id}" title="Complete">✓</button>`}
      <button class="btn btn-sm btn-danger" data-del="${i.id}" title="Delete">×</button>
    </div>`;
  }

  document.getElementById("inbox-add")?.addEventListener("click", addItem);
  document.getElementById("inbox-text")?.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") addItem();
  });

  document.querySelectorAll("[data-done]").forEach(el => {
    el.addEventListener("click", async () => {
      try {
        await call("update_inbox_item", { id: Number(el.dataset.done), input: { completed: true } });
        render(container);
      } catch (e) { errorToast(e); }
    });
  });
  document.querySelectorAll("[data-reopen]").forEach(el => {
    el.addEventListener("click", async () => {
      try {
        await call("update_inbox_item", { id: Number(el.dataset.reopen), input: { completed: false } });
        render(container);
      } catch (e) { errorToast(e); }
    });
  });
  document.querySelectorAll("[data-del]").forEach(el => {
    el.addEventListener("click", async () => {
      try {
        await call("delete_inbox_item", { id: Number(el.dataset.del) });
        render(container);
      } catch (e) { errorToast(e); }
    });
  });
  document.querySelectorAll("[data-promote]").forEach(el => {
    el.addEventListener("click", async () => {
      const id = Number(el.dataset.promote);
      const item = items.find(i => i.id === id);
      if (item) await promoteItem(container, item);
    });
  });
}

async function addItem() {
  const text = document.getElementById("inbox-text")?.value.trim();
  if (!text) return;
  try {
    await call("create_inbox_item", { input: { text } });
    const { toast } = await import("./app.js");
    toast("Captured");
    const container = document.getElementById("content");
    await render(container);
  } catch (e) { errorToast(e); }
}

async function promoteItem(container, item) {
  let projects = [];
  try { projects = await loadProjects(); } catch {}
  const { openModal, closeModal, buildModal, toast } = await import("./app.js");
  openModal("task-modal", buildModal(
    "task-modal",
    `<h2>Promote to task</h2>`,
    `<div class="stack">
       <div class="field"><label>Title</label><input id="pm-title" class="input" value="${esc(item.text)}"></div>
       <div class="field"><label>Project</label><select id="pm-project" class="select">
         <option value="">Select project…</option>
         ${projects.map(p => `<option value="${p.id}">${esc(p.key)} — ${esc(p.name)}</option>`).join("")}
       </select></div>
     </div>`,
    `<button class="btn" data-close="task-modal">Cancel</button>
     <button class="btn btn-primary" id="pm-save">Create task</button>`
  ));
  document.getElementById("pm-save").addEventListener("click", async () => {
    const project_id = Number(document.getElementById("pm-project").value);
    const title = document.getElementById("pm-title").value.trim();
    if (!project_id || !title) { toast("Project and title are required.", true); return; }
    try {
      const task = await call("create_task", { input: { project_id, title } });
      await call("update_inbox_item", {
        id: item.id,
        input: { completed: true, promoted_task_id: task.id },
      });
      closeModal("task-modal");
      toast(`Promoted → ${task.key}`);
      await render(container);
      const { openTaskDetail } = await import("./app.js");
      openTaskDetail(task.id);
    } catch (e) { toast(e.message, true); }
  });
}

function errorToast(e) {
  import("./app.js").then(({ toast }) => toast(e.message, true));
}