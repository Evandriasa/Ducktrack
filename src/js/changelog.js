import { call } from "./api.js";
import { esc, fmtDay, statusBadge } from "./ui.js";

export async function render(container) {
  let history = [];
  try {
    history = await call("get_global_history", { filter: { limit: 300 } });
  } catch {}

  // group by day
  const groups = {};
  (history || []).forEach(h => {
    const day = fmtDay(h.createdAt);
    if (!groups[day]) groups[day] = [];
    groups[day].push(h);
  });

  const dayHtml = Object.entries(groups).map(([day, entries]) => {
    const rows = entries.map(h => {
      const desc = h.action === "created"
        ? `created`
        : h.action === "completed"
          ? `<span class="small" style="color:var(--green)">Completed</span>`
          : h.action === "cancelled"
            ? `<span class="small" style="color:var(--red)">Cancelled</span>`
            : h.action === "reopened"
              ? `<span class="small" style="color:var(--amber)">Reopened</span>`
              : h.field === "status"
                ? `${esc(h.oldValue || "—")} → <b>${esc(h.newValue || "—")}</b>`
                : h.field
                  ? `${esc(h.field)}: ${esc(h.newValue || "—")}`
                  : esc(h.action);
      return `<div class="row-item">
        <span class="key" style="width:90px">${esc(h.taskKey)}</span>
        <div class="grow">
          <div class="small">${esc(h.taskTitle)}</div>
          <div><span class="xs muted">${esc(h.projectKey)}</span></div>
        </div>
        <div class="small muted" style="min-width:180px;text-align:right">${desc}</div>
      </div>`;
    }).join("");
    return `<div class="mb-3">
      <div class="eyebrow" style="margin:16px 0 0">${esc(day)}</div>
      <div class="card" style="overflow:hidden">${rows}</div>
    </div>`;
  }).join("");

  container.innerHTML = `
    <div class="page-max" style="max-width:1100px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Immutable history</div>
          <h1 class="page-title">Change Log</h1>
          <p class="page-desc">A permanent record of what changed, when and why. Generated automatically.</p>
        </div>
      </div>
      ${dayHtml || '<div class="empty"><div class="big">No changes recorded yet</div></div>'}
    </div>`;
}