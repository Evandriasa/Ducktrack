import { call } from "./api.js";
import { esc, fmtDay, fmtTime, fmtDur, dot, statusBadge, renderTextWithMentions, todayLocal, isoDaysAgo } from "./ui.js";

const DAYS_AGO = 6;
let currentDate = todayLocal();

export async function render(container) {
  const today = currentDate;
  const from = isoDaysAgo(DAYS_AGO);

  let summary = null, report = null, blocked = [];
  try { summary = await call("get_daily_summary", { date: today }); } catch {}
  try { report = await call("get_summary_report", { from, to: today }); } catch {}
  try { blocked = await call("get_blocked_tasks"); } catch {}

  const completedToday = (summary?.history || []).filter(h => h.action === "completed").length;

  container.innerHTML = `
    <div class="page-max" style="max-width:980px">
      <div class="page-head">
        <div>
          <div class="eyebrow">What actually happened</div>
          <h1 class="page-title">Journal</h1>
          <p class="page-desc">A day-by-day record of work logs, changes and blockages.</p>
        </div>
      </div>

      <div class="card mb-3">
        <div class="card-head">
          <div><h3>Daily summary</h3><div class="sub">Work logged and task changes for the selected day</div></div>
          <input id="journal-date" class="input" type="date" value="${esc(today)}" style="width:auto">
        </div>
        <div class="stats" style="padding:16px">
          <div class="card card-pad stat"><div class="label">Time logged</div><div class="value">${fmtDur(summary?.minutes || 0)}</div></div>
          <div class="card card-pad stat"><div class="label">Log entries</div><div class="value">${summary?.logCount || 0}</div></div>
          <div class="card card-pad stat"><div class="label">Tasks completed</div><div class="value" style="color:var(--green)">${completedToday}</div></div>
          <div class="card card-pad stat"><div class="label">Changes</div><div class="value">${summary?.history?.length || 0}</div></div>
        </div>
      </div>

      <div class="grid" style="grid-template-columns:1fr 1fr;gap:20px">
        <div class="card" style="overflow:hidden">
          <div class="card-head"><div><h3>Work logs</h3><div class="sub">Time captured on that day</div></div></div>
          <div id="journal-logs">${logsHtml(summary)}</div>
        </div>
        <div class="card" style="overflow:hidden">
          <div class="card-head"><div><h3>Timeline</h3><div class="sub">Task history that day</div></div></div>
          <div id="journal-timeline">${timelineHtml(summary)}</div>
        </div>
      </div>

      <div class="card mt-3 mb-3" style="overflow:hidden">
        <div class="card-head">
          <div><h3>Period report</h3><div class="sub">Hours and completions across a date range</div></div>
          <div class="row" style="gap:8px">
            <input id="report-from" class="input" type="date" value="${esc(from)}" style="width:auto">
            <span class="xs muted">→</span>
            <input id="report-to" class="input" type="date" value="${esc(today)}" style="width:auto">
          </div>
        </div>
        ${reportHtml(report)}
      </div>

      <div class="card card-pad" style="overflow:hidden">
        <div class="card-head" style="padding:0 0 12px"><div><h3 style="color:var(--red)">Blocked tasks</h3><div class="sub">Stuck work and what is keeping it stuck</div></div></div>
        <div id="journal-blocked">${blockedHtml(blocked)}</div>
      </div>
    </div>`;

  document.getElementById("journal-date")?.addEventListener("change", async (e) => {
    if (!e.target.value) return;
    currentDate = e.target.value;
    await render(container);
  });

  document.getElementById("report-from")?.addEventListener("change", reloadReport);
  document.getElementById("report-to")?.addEventListener("change", reloadReport);

  async function reloadReport() {
    const f = document.getElementById("report-from").value;
    const t = document.getElementById("report-to").value;
    if (!f || !t) return;
    try {
      const r = await call("get_summary_report", { from: f, to: t });
      const el = document.querySelector(".period-report");
      if (el) el.outerHTML = reportHtml(r);
    } catch (e) { reportError(e); }
  }

  function reportError(e) {
    import("./app.js").then(({ toast }) => toast(e.message, true));
  }
}

function logsHtml(summary) {
  const logs = summary?.workLogs || [];
  if (!logs.length) return `<div class="empty" style="padding:24px"><div class="big">No work logged</div></div>`;
  return logs.map(l => {
    const wl = l.work_log || l;
    return `<div class="row-item">
      <span class="key" style="width:80px">${esc(l.taskKey || "")}</span>
      <div class="grow"><div class="small">${renderTextWithMentions(wl.description)}</div><div class="xs muted mt-1">${fmtTime(wl.createdAt)}</div></div>
      <span class="xs" style="white-space:nowrap">${fmtDur(wl.durationMinutes)}</span>
    </div>`;
  }).join("");
}

function timelineHtml(summary) {
  const h = summary?.history || [];
  if (!h.length) return `<div class="empty" style="padding:24px"><div class="big">No changes</div></div>`;
  return `<div class="timeline" style="padding:18px 20px">
    ${h.map(entry => {
      let detail = "";
      if (entry.action === "created") detail = `Status: ${esc(entry.newValue || "")}`;
      else if (entry.field === "status") detail = `${esc(entry.oldValue || "—")} → ${esc(entry.newValue || "—")}`;
      else if (entry.field) detail = `<span class="ghost-big">${esc(entry.field)}</span> ${esc(entry.oldValue || "—")} → ${esc(entry.newValue || "—")}`;
      else detail = esc(entry.action);
      return `<div class="stack mb-0">
        <div class="tl-time">${fmtTime(entry.createdAt)} · <span class="key">${esc(entry.taskKey)}</span></div>
        <div class="tl-action">${statusBadge(entry.action)}</div>
        <div class="tl-detail">${detail}</div>
      </div>`;
    }).join("")}
  </div>`;
}

function reportHtml(report) {
  if (!report) return `<div class="empty" style="padding:24px"><div class="big">Report unavailable</div></div>`;
  const hours = (report.minutes / 60).toFixed(1);
  const rows = (report.byProject || []).map(p =>
    `<tr><td><span class="key">${esc(p.projectKey)}</span> ${esc(p.projectName)}</td><td>${p.logs}</td><td>${fmtDur(p.minutes)}</td><td>${(p.minutes / 60).toFixed(1)}h</td></tr>`
  ).join("") || `<tr><td colspan="4" class="muted">No work logged in this period</td></tr>`;
  return `<div class="period-report card-pad">
    <div class="stats" style="margin-bottom:14px">
      <div class="card card-pad stat"><div class="label">Hours logged</div><div class="value">${hours}h</div></div>
      <div class="card card-pad stat"><div class="label">Log entries</div><div class="value">${report.logCount}</div></div>
      <div class="card card-pad stat"><div class="label">Tasks completed</div><div class="value" style="color:var(--green)">${report.tasksCompleted}</div></div>
    </div>
    <div class="xs muted mb-2">By project</div>
    <table class="journal-table">
      <thead><tr><th>Project</th><th>Logs</th><th>Time</th><th>Hours</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  </div>`;
}

function blockedHtml(blocked) {
  if (!blocked.length) return `<div class="xs muted">Nothing is blocked right now.</div>`;
  return blocked.map(b => `
    <div class="row-item" style="cursor:pointer" onclick="window.openTask(${b.taskId})">
      ${dot("Blocked")}
      <span class="key" style="width:90px">${esc(b.key)}</span>
      <span class="small grow">${esc(b.title)}</span>
      <span class="xs muted">${esc(b.projectKey)}</span>
      <span class="xs muted">${(b.blockers || []).map(k => `<span class="journal-chip">blocked by ${esc(k)}</span>`).join("") || "no blocking relations"}</span>
    </div>`).join("");
}