import { call, loadProjects } from "./api.js";
import { esc, fmtDay, projectOptions } from "./ui.js";

const FW_STATUSES = ["Open", "Investigating", "Actioned", "Closed"];

const FW_CLS = (s) => ({
  Open: "b-todo",
  Investigating: "b-in-progress",
  Actioned: "b-completed",
  Closed: "b-cancelled",
}[s] || "b-todo");

function chainLevels(p) {
  const levels = [];
  for (let i = 1; i <= 5; i++) {
    const text = (p[`why${i}`] || "").trim();
    if (text) levels.push(text);
  }
  return levels;
}

export function fiveWhyBadge(status) {
  return `<span class="badge ${FW_CLS(status)}">${esc(status)}</span>`;
}

export async function render(container) {
  let analyses = [];
  let projects = [];
  try { analyses = await call("list_five_whys"); } catch {}
  try { projects = await loadProjects(); } catch {}

  container.innerHTML = `
    <div class="page-max" style="max-width:1200px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Root cause analysis</div>
          <h1 class="page-title">5-Why</h1>
          <p class="page-desc">Systematic root cause analysis for recurring problems.</p>
        </div>
        <button class="btn btn-primary" id="fw-new">+ New analysis</button>
      </div>
      <div class="grid" style="grid-template-columns:repeat(2,1fr);gap:16px;align-items:start">
        ${(analyses || []).map(card).join("") || '<div class="card card-pad" style="grid-column:1/-1"><div class="empty"><div class="big">No analyses yet</div><p class="small muted mt-2">Start a 5-Why to track the root cause of a problem and its corrective actions.</p></div></div>'}
      </div>
    </div>`;

  document.getElementById("fw-new")?.addEventListener("click", () => openEditor(null, projects, container));

  document.querySelectorAll("[data-fw-edit]").forEach(b => b.addEventListener("click", (e) => {
    e.stopPropagation();
    const id = Number(b.dataset.fwEdit);
    const p = (analyses || []).find(x => x.id === id);
    if (p) openEditor(p, projects, container);
  }));

  document.querySelectorAll("[data-fw-del]").forEach(b => b.addEventListener("click", (e) => {
    e.stopPropagation();
    const id = Number(b.dataset.fwDel);
    const p = (analyses || []).find(x => x.id === id);
    if (p) confirmDelete(p, container);
  }));

  document.querySelectorAll("[data-fw-task]").forEach(b => b.addEventListener("click", async (e) => {
    e.stopPropagation();
    const id = Number(b.dataset.fwTask);
    const p = (analyses || []).find(x => x.id === id);
    if (p) await createTaskFromAction(p, projects, container);
  }));
}

function card(p) {
  const levels = chainLevels(p);
  const action = (p.correctiveAction || "").trim();
  const linked = p.taskKey ? `<span class="chip">→ ${esc(p.taskKey)}</span>` : "";
  return `
    <div class="card card-pad" style="min-width:0">
      <div class="spread mb-2">
        <div class="row" style="gap:8px">
          ${fiveWhyBadge(p.status)}
          ${p.projectKey ? `<span class="chip">${esc(p.projectKey)}${linked}</span>` : linked}
        </div>
        <div class="xs muted">#${p.id} · ${esc(p.createdAt ? fmtDay(p.createdAt) : "")}</div>
      </div>
      <div class="fw-problem">${esc(p.problem)}</div>
      <div class="mt-3 fw-chain">
        ${levels.map((why, i) => `
          <div class="fw-node">
            <span class="fw-num">Why ${i + 1}</span>
            <span class="fw-text">${esc(why)}</span>
          </div>`).join("")}
      </div>
      ${(p.rootCause || "").trim() ? `
        <div class="fw-root mt-3">
          <div class="xs muted" style="text-transform:uppercase;letter-spacing:.06em">Root cause</div>
          <div class="mt-1">${esc(p.rootCause)}</div>
        </div>` : ""}
      ${action ? `
        <div class="fw-root fw-action mt-3">
          <div class="xs muted" style="text-transform:uppercase;letter-spacing:.06em">Corrective action</div>
          <div class="mt-1">${esc(action)}</div>
        </div>` : ""}
      <div class="row mt-3" style="justify-content:space-between">
        <div class="row" style="gap:10px">
          ${p.owner ? `<span class="xs muted">👤 ${esc(p.owner)}</span>` : ""}
          ${p.dueDate ? `<span class="xs muted">⏱ ${esc(p.dueDate)}</span>` : ""}
        </div>
        <div class="row" style="gap:6px">
          ${action ? `<button class="btn btn-sm" data-fw-task="${p.id}">+ Task</button>` : ""}
          <button class="btn btn-sm" data-fw-edit="${p.id}">Edit</button>
          <button class="btn btn-sm btn-danger" data-fw-del="${p.id}">Delete</button>
        </div>
      </div>
    </div>`;
}

function buildOptions(projects, selected) {
  return `<option value="">— No project —</option>` + projectOptions(projects, selected);
}

async function openEditor(record, projects, container) {
  const { openModal, closeModal, buildModal, toast } = await import("./app.js");
  const isEdit = !!record;

  const val = (k) => esc(record ? (record[k] || "") : "");

  openModal("fivewhy-modal", buildModal(
    "fivewhy-modal",
    `<h2>${isEdit ? "Edit 5-Why #" + record.id : "New 5-Why analysis"}</h2>`,
    `<div class="stack">
       <div class="field">
         <label>Problem</label>
         <textarea id="fw-problem" class="textarea" placeholder="What happened, and why is it important?">${val("problem")}</textarea>
       </div>
       <div class="row">
         <div class="field grow">
           <label>Project</label>
           <select id="fw-project" class="select">${buildOptions(projects, record?.projectId)}</select>
         </div>
         <div class="field grow">
           <label>Linked task key</label>
           <input id="fw-task-key" class="input" placeholder="e.g. AGV-042" value="${val("taskKey")}">
         </div>
       </div>
       <div class="field">
         <label>Why chain</label>
         ${[1,2,3,4,5].map(n => `
           <input id="fw-why-${n}" class="input mb-1" placeholder="Why ${n}" value="${val("why" + n)}">`).join("")}
       </div>
       <div class="field"><label>Root cause</label><textarea id="fw-root" class="textarea" placeholder="The underlying cause">${val("rootCause")}</textarea></div>
       <div class="field"><label>Corrective action</label><textarea id="fw-action" class="textarea" placeholder="What will be done to prevent recurrence?">${val("correctiveAction")}</textarea></div>
       <div class="row">
         <div class="field grow"><label>Owner</label><input id="fw-owner" class="input" value="${val("owner")}" placeholder="Responsible person"></div>
         <div class="field grow"><label>Due date</label><input id="fw-due" class="input" type="date" value="${val("dueDate")}"></div>
         <div class="field grow"><label>Status</label><select id="fw-status" class="select">
           ${FW_STATUSES.map(s => `<option value="${s}" ${record?.status === s ? "selected" : ""}>${s}</option>`).join("")}
         </select></div>
       </div>
     </div>`,
    `<button class="btn" data-close="fivewhy-modal">Cancel</button>
     <button class="btn btn-primary" id="fw-save">${isEdit ? "Save changes" : "Create analysis"}</button>`
  ));

  document.getElementById("fw-save").addEventListener("click", async () => {
    const problem = document.getElementById("fw-problem").value.trim();
    if (!problem) { toast("Problem is required.", true); return; }

    const projectId = document.getElementById("fw-project").value;
    const taskKey = (document.getElementById("fw-task-key").value || "").trim();

    let taskId = record?.taskId || undefined;
    if (taskKey) {
      try {
        const t = await call("get_task_by_key", { key: taskKey });
        if (t) taskId = t.id;
        else { toast(`Task "${taskKey}" not found.`, true); return; }
      } catch {
        toast(`Task "${taskKey}" not found.`, true);
        return;
      }
    } else if (isEdit) {
      taskId = undefined;
    }

    const why = {};
    for (let n = 1; n <= 5; n++) {
      const v = document.getElementById(`fw-why-${n}`)?.value.trim();
      if (v) why[`why_${n}`] = v;
    }

    const payload = {
      problem,
      project_id: projectId ? Number(projectId) : null,
      task_id: taskId ?? null,
      root_cause: document.getElementById("fw-root")?.value.trim(),
      corrective_action: document.getElementById("fw-action")?.value.trim(),
      owner: document.getElementById("fw-owner")?.value.trim(),
      due_date: document.getElementById("fw-due")?.value,
      status: document.getElementById("fw-status")?.value,
      ...why,
    };

    try {
      if (isEdit) {
        await call("update_five_why", { id: record.id, input: payload });
        toast("Analysis updated");
      } else {
        await call("create_five_why", { input: payload });
        toast("Analysis created");
      }
      closeModal("fivewhy-modal");
      await render(container);
    } catch (e) {
      toast(e.message, true);
    }
  });
}

async function confirmDelete(p, container) {
  const { confirmDialog, toast } = await import("./app.js");
  if (!(await confirmDialog(`Delete 5-Why #${p.id}?`, { title: "Delete analysis", confirmLabel: "Delete" }))) return;
  try {
    await call("delete_five_why", { id: p.id });
    toast("Analysis deleted");
    await render(container);
  } catch (e) {
    toast(e.message, true);
  }
}

async function createTaskFromAction(p, projects, container) {
  const { openModal, closeModal, buildModal, toast } = await import("./app.js");
  const action = (p.correctiveAction || "").trim();
  if (!action) { toast("This analysis has no corrective action.", true); return; }
  const selected = projects.length === 1 ? projects[0].id : (p.projectId || undefined);

  openModal("fivewhy-task-modal", buildModal(
    "fivewhy-task-modal",
    `<h2>Create task from corrective action</h2>`,
    `<div class="stack">
       <div class="field">
         <label>Project</label>
         <select id="fwt-project" class="select">${buildOptions(projects, selected)}</select>
       </div>
       <div class="field"><label>Title</label><input id="fwt-title" class="input" value="${esc(action)}"></div>
       <div class="field"><label>Description</label><textarea id="fwt-desc" class="textarea">Source: 5-WHY #${p.id}\nProblem: ${esc(p.problem)}\nRoot cause: ${esc(p.rootCause || '')}</textarea></div>
       <div class="row">
         <div class="field grow"><label>Priority</label><select id="fwt-prio" class="select">
           <option value="Medium">Medium</option><option value="Low">Low</option><option value="High">High</option><option value="Critical">Critical</option>
         </select></div>
         <div class="field grow"><label>Status</label><select id="fwt-status" class="select">
           <option value="Todo">Todo</option><option value="In Progress">In Progress</option><option value="Blocked">Blocked</option>
         </select></div>
       </div>
     </div>`,
    `<button class="btn" data-close="fivewhy-task-modal">Cancel</button>
     <button class="btn btn-primary" id="fwt-save">Create task</button>`
  ));

  document.getElementById("fwt-save").addEventListener("click", async () => {
    const projectId = Number(document.getElementById("fwt-project").value);
    const title = document.getElementById("fwt-title").value.trim();
    if (!projectId) { toast("Select a project.", true); return; }
    if (!title) { toast("Title is required.", true); return; }
    try {
      const created = await call("create_task", {
        input: {
          project_id: projectId,
          title,
          description: document.getElementById("fwt-desc").value,
          priority: document.getElementById("fwt-prio").value,
          status: document.getElementById("fwt-status").value,
          task_type: "Task",
        },
      });
      toast(`Task ${created.key} created`);
      closeModal("fivewhy-task-modal");
      try {
        await call("update_five_why", {
          id: p.id,
          input: { taskId: created.id },
        });
      } catch {}
      await render(container);
    } catch (e) {
      toast(e.message, true);
    }
  });
}