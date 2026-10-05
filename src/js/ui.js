export const STATUSES = ["Todo", "In Progress", "Blocked", "Completed", "Cancelled"];
export const PRIORITIES = ["Low", "Medium", "High", "Critical"];
export const TYPES = ["Task", "Bug", "Feature", "Improvement", "Maintenance", "Investigation", "Documentation"];

export function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[c]));
}

export function fmtDate(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d)) return iso;
  return d.toLocaleString(undefined, {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function fmtDay(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d)) return iso;
  return d.toLocaleDateString(undefined, { day: "2-digit", month: "short", year: "numeric" });
}

export function fmtTime(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d)) return iso;
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function fmtDur(mins) {
  if (!mins) return "";
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return h ? `${h}h ${m}m` : `${m}m`;
}

export function fmtBytes(bytes) {
  if (!bytes && bytes !== 0) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const SLUG = (s) => (s || "").toLowerCase().replace(/[^a-z]+/g, "-");
export const dotCls = (s) => ({
  "In Progress": "st-progress",
  Todo: "st-todo",
  Blocked: "st-blocked",
  Completed: "st-done",
  Cancelled: "st-cancelled",
}[s] || "st-todo");

export function dot(s) {
  return `<span class="dot ${dotCls(s)}"></span>`;
}

export function statusBadge(s) {
  return `<span class="badge b-${SLUG(s)}">${esc(s)}</span>`;
}

export function renderTextWithMentions(text) {
  return esc(text).replace(/@@([A-Z0-9]+-\d+)/gi, (m, key) => {
    return `<span class="mention" data-task-key="${esc(key.toUpperCase())}" title="Open task ${esc(key.toUpperCase())}">${m}</span>`;
  });
}

export function todayLocal() {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function isoDaysAgo(days) {
  const d = new Date(Date.now() - days * 86400000);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function prioClass(p) {
  return `p-${SLUG(p || "Medium")}`;
}

export function prio(p) {
  return `<span class="${prioClass(p)}">${esc(p)}</span>`;
}

export function typeBadge(t) {
  return `<span class="badge b-todo">${esc(t)}</span>`;
}

export function projectOptions(projects, selected) {
  return projects
    .map((p) => `<option value="${p.id}" ${p.id === selected ? "selected" : ""}>${esc(p.key)} — ${esc(p.name)}</option>`)
    .join("");
}

export function loading(el) {
  if (el) el.innerHTML = `<div class="empty"><div class="big">Loading…</div></div>`;
}