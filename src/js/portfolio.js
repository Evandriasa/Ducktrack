import { call, loadProjects } from "./api.js";
import { esc } from "./ui.js";

export async function render(container) {
  let stats, projects, tasks, logs;
  try {
    [stats, projects, tasks, logs] = await Promise.all([
      call("get_dashboard_stats"),
      loadProjects(),
      call("list_tasks", { filter: { include_subtasks: false } }),
      call("list_work_logs"),
    ]);
  } catch {}

  const typeCounts = {};
  const projectCounts = {};
  (tasks || []).forEach(t => {
    const task = t.task || t;
    if (task.status === "Completed") {
      const type = task.taskType || "Task";
      typeCounts[type] = (typeCounts[type] || 0) + 1;
      const pname = t.projectName || "Unknown";
      projectCounts[pname] = (projectCounts[pname] || 0) + 1;
    }
  });

  const features = (typeCounts["Feature"] || 0) + (typeCounts["Improvement"] || 0);
  const bugs = typeCounts["Bug"] || 0;
  const completedHours = (logs || []).reduce((a, l) => {
    const wl = l.work_log || l;
    return a + (wl.durationMinutes || 0);
  }, 0);

  const topProjects = Object.entries(projectCounts).sort((a, b) => b[1] - a[1]).slice(0, 4);

  container.innerHTML = `
    <div class="page-max" style="max-width:1200px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Proof of work</div>
          <h1 class="page-title">Portfolio</h1>
          <p class="page-desc">A clean, presentable view of completed engineering work — generated from real history.</p>
        </div>
        <div class="row" style="gap:8px">
          <button class="btn" id="pf-export">Export workspace</button>
          <button class="btn" id="pf-excel">Excel (XLSX)</button>
          <button class="btn" id="pf-excel-import">Import Excel</button>
          <button class="btn btn-primary" id="pf-portfolio">Portfolio (MD + HTML)</button>
        </div>
      </div>
      <div class="card card-pad mb-3">
        <div class="spread">
          <div><div class="text-xl" style="font-size:20px;font-weight:600">Engineering Work Overview</div><div class="small muted mt-1">A summary generated from your actual project history.</div></div>
        </div>
        <div class="grid" style="grid-template-columns:repeat(4,1fr);gap:20px;margin-top:28px">
          <div><div class="value" style="font-size:32px;font-weight:600;color:var(--green)">${stats ? stats.completedTasks : 0}</div><div class="xs muted">Completed tasks</div></div>
          <div><div class="value" style="font-size:32px;font-weight:600;color:var(--accent)">${features}</div><div class="xs muted">Features / improvements</div></div>
          <div><div class="value" style="font-size:32px;font-weight:600;color:var(--red)">${bugs}</div><div class="xs muted">Bug fixes</div></div>
          <div><div class="value" style="font-size:32px;font-weight:600">${Math.round(completedHours / 60)}</div><div class="xs muted">Hours logged</div></div>
        </div>
      </div>
      <div class="grid" style="grid-template-columns:repeat(2,1fr);gap:16px">
        ${topProjects.map(([name, count]) => `
          <div class="card card-pad">
            <div class="spread"><span class="small" style="font-weight:600">${esc(name)}</span><span class="dot st-done"></span></div>
            <div class="xs muted mt-1">${count} completed tasks</div>
          </div>`).join("") || '<div class="empty">Complete tasks to build your portfolio</div>'}
      </div>
    </div>`;

  document.getElementById("pf-export")?.addEventListener("click", async () => {
    try {
      const path = await call("export_workspace");
      const { toast } = await import("./app.js");
      toast(`Workspace exported to ${path}`);
    } catch (e) {
      const { toast } = await import("./app.js");
      toast(e.message, true);
    }
  });

  document.getElementById("pf-excel")?.addEventListener("click", async () => {
    try {
      const path = await call("export_excel");
      const { toast, openModal, buildModal } = await import("./app.js");
      openModal("task-modal", buildModal(
        "task-modal",
        `<h2>Excel export ready</h2>`,
        `<div class="stack">
          <div class="field"><label>File</label><div class="mono xs" style="word-break:break-all;color:var(--accent)">${esc(path)}</div></div>
          <p class="xs muted mt-2">The workbook has styled sheets: Overview, Projects, Tasks, Work Logs, Documents, Inbox and 5-Why. Edit the Projects and Tasks sheets in Excel, then use “Import Excel” to sync them back.</p>
        </div>`,
        `<button class="btn btn-primary" data-close="task-modal">Done</button>`
      ));
    } catch (e) {
      const { toast } = await import("./app.js");
      toast(e.message, true);
    }
  });

  document.getElementById("pf-excel-import")?.addEventListener("click", async () => {
    const { openModal, closeModal, buildModal, toast, confirmDialog } = await import("./app.js");
    openModal("task-modal", buildModal(
      "task-modal",
      `<h2>Import from Excel</h2>`,
      `<div class="stack">
        <p class="small">Paste the path to a DuckTrack .xlsx workbook (from “Excel (XLSX)” export). The <b>Projects</b> and <b>Tasks</b> sheets are imported by Key: new rows are created, existing rows are updated.</p>
        <div class="field mt-2"><label>Workbook path</label><input id="xl-import-path" class="input" placeholder="/path/to/ducktrack-workspace.xlsx"></div>
      </div>`,
      `<button class="btn" data-close="task-modal">Cancel</button>
       <button class="btn btn-primary" id="xl-import-go">Import</button>`
    ));
    document.getElementById("xl-import-go").addEventListener("click", async () => {
      const p = document.getElementById("xl-import-path").value.trim();
      if (!p) { toast("Enter the workbook path.", true); return; }
      if (!(await confirmDialog(`Import ${p}?`, { title: "Import Excel", confirmLabel: "Import", danger: false }))) return;
      try {
        const r = await call("import_excel", { path: p });
        closeModal("task-modal");
        toast(`Imported: ${r.projectsCreated} projects, ${r.projectsUpdated} updated, ${r.tasksCreated} tasks, ${r.tasksUpdated} updated${r.skippedMissingProject ? ` (${r.skippedMissingProject} skipped)` : ""}`);
      } catch (e) {
        toast(e.message, true);
      }
    });
  });

  document.getElementById("pf-portfolio")?.addEventListener("click", async () => {
    try {
      const r = await call("export_portfolio");
      const { toast, openModal, buildModal } = await import("./app.js");
      openModal("task-modal", buildModal(
        "task-modal",
        `<h2>Portfolio exported</h2>`,
        `<div class="stack">
          <div class="field"><label>Markdown</label><div class="mono xs" style="word-break:break-all;color:var(--accent)">${esc(r.markdownPath)}</div></div>
          <div class="field mt-2"><label>HTML</label><div class="mono xs" style="word-break:break-all;color:var(--accent)">${esc(r.htmlPath)}</div></div>
          <p class="xs muted mt-2">Open the HTML file in a browser to see your ready-to-share portfolio.</p>
        </div>`,
        `<button class="btn btn-primary" data-close="task-modal">Done</button>`
      ));
    } catch (e) {
      const { toast } = await import("./app.js");
      toast(e.message, true);
    }
  });
}