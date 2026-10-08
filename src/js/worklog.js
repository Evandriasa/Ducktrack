import { call, loadProjects } from "./api.js";
import { esc, fmtDay, fmtDur, projectOptions } from "./ui.js";

export async function render(container) {
  let logs = [];
  let projects = [];
  try {
    [logs, projects] = await Promise.all([
      call("list_work_logs"),
      loadProjects(),
    ]);
  } catch {}

  let editingId = null;

  function fillForm(l) {
    const wl = l.work_log || l;
    document.getElementById("wl-text").value = wl.description || "";
    document.getElementById("wl-dur").value = wl.durationMinutes || "";
    document.getElementById("wl-project").value = (wl.projectId ?? wl.project_id) || "";
    document.getElementById("wl-save").textContent = "Save changes";
    editingId = wl.id;
  }

  const entries = (logs || []).map(l => {
    // WorkLogWithRefs: worklog fields are flattened
    const wl = l.work_log || l;
    const ref = l.projectName ? `${esc(l.taskKey ? l.taskKey + " · " : "")}${esc(l.projectName)}` : "";
    return `<div class="card card-pad">
      <div class="spread">
        <div><span class="xs muted">${fmtDay(wl.createdAt)}</span> <span class="small muted">·</span> <span class="small" style="color:var(--accent)">${ref || 'General'}</span></div>
        <div class="row" style="gap:8px">
          <span class="xs muted">${wl.durationMinutes ? esc(fmtDur(wl.durationMinutes)) : ""}</span>
          <button class="btn btn-sm" data-edit-log="${wl.id}">Edit</button>
          <button class="btn btn-sm btn-danger" data-del-log="${wl.id}" title="Delete">&times;</button>
        </div>
      </div>
      <p class="small" style="line-height:1.7;margin-top:10px;color:var(--text)">${esc(wl.description)}</p>
    </div>`;
  }).join("");

  container.innerHTML = `
    <div class="page-max" style="max-width:1100px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Engineering journal</div>
          <h1 class="page-title">Work Log</h1>
          <p class="page-desc">Record the work that does not fit neatly into a task.</p>
        </div>
      </div>
      <div class="card card-pad mb-3">
        <div class="stack">
          <div class="field"><label>Description</label><textarea id="wl-text" class="textarea" rows="4" placeholder="Describe what you worked on, what you found and what the result was…"></textarea></div>
          <div class="grid-3">
            <div class="field"><label>Project</label><select id="wl-project" class="select"><option value="">General</option>${projectOptions(projects)}</select></div>
            <div class="field"><label>Duration (minutes)</label><input id="wl-dur" class="input" type="number" placeholder="optional"></div>
            <div class="field"><label>&nbsp;</label><button class="btn btn-primary" id="wl-save">Add work log</button></div>
          </div>
        </div>
      </div>
      <div class="stack" style="gap:14px">${entries || '<div class="empty"><div class="big">No work logged yet</div></div>'}</div>
    </div>`;

  document.getElementById("wl-save")?.addEventListener("click", async () => {
    const text = document.getElementById("wl-text").value.trim();
    const dur = Number(document.getElementById("wl-dur").value) || null;
    const pid = Number(document.getElementById("wl-project").value) || null;
    if (!text) return;
    try {
      const { toast } = await import("./app.js");
      if (editingId) {
        // 0 project = "General" (NULL), 0 duration = cleared — backend turns these into NULL
        await call("update_work_log", { id: editingId, input: { description: text, duration_minutes: Number(document.getElementById("wl-dur").value) || 0, project_id: Number(document.getElementById("wl-project").value) || 0 } });
        toast("Work log updated");
      } else {
        await call("create_work_log", { input: { description: text, duration_minutes: dur, project_id: pid } });
        toast("Work log saved");
      }
      editingId = null;
      await render(container);
    } catch (e) {
      const { toast } = await import("./app.js");
      toast(e.message, true);
    }
  });

  document.querySelectorAll("[data-edit-log]").forEach(btn => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const id = Number(btn.dataset.editLog);
      const l = (logs || []).find(x => (x.work_log || x).id === id);
      if (l) fillForm(l);
    });
  });

  document.querySelectorAll("[data-del-log]").forEach(btn => {
    btn.addEventListener("click", async (e) => {
      e.stopPropagation();
      const id = Number(btn.dataset.delLog);
      const { toast, confirmDialog } = await import("./app.js");
      if (!(await confirmDialog("Delete this work log entry?", { title: "Delete work log" }))) return;
      try {
        await call("delete_work_log", { id });
        toast("Work log deleted");
        await render(container);
      } catch (err) {
        toast(err.message, true);
      }
    });
  });
}