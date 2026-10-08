/* EMBEDMARKER-v1 */
import { call, loadProjects, loadProjectTree, loadTasks } from "./api.js";
import * as dashboard from "./dashboard.js";
import * as tasks from "./tasks.js";
import * as inbox from "./inbox.js";
try { await call("debug_log", { msg: "app.js: imports finished, module body START" }); } catch {}
window.addEventListener("error", (e) => { try { call("debug_log", { msg: "window.onerror: " + (e.message||"") + "@" + (e.filename||"") + ":" + (e.lineno||"") }); } catch {} });
window.addEventListener("unhandledrejection", (e) => { try { call("debug_log", { msg: "unhandledrejection: " + String((e.reason && e.reason.message) || e.reason) }); } catch {} });
import * as projects from "./projects.js";
import * as changelog from "./changelog.js";
import * as journal from "./journal.js";
import * as worklog from "./worklog.js";
import * as documents from "./documents.js";
import * as portfolio from "./portfolio.js";
import * as settings from "./settings.js";
import * as fivewhy from "./fivewhy.js";
import * as help from "./help.js";
import { esc, STATUSES, PRIORITIES, TYPES, fmtDay, fmtTime, fmtDur, statusBadge, fmtBytes, renderTextWithMentions, todayLocal } from "./ui.js";
import { renderMarkdown } from "./markdown.js";

const PAGE_DEFS = [
  { id: "dashboard", label: "Dashboard", icon: "⌂", page: dashboard },
  { id: "inbox", label: "Inbox", icon: "✉", page: inbox },
  { id: "tasks", label: "My Tasks", icon: "✓", page: tasks, badgeId: "tasks-badge" },
  { id: "projects", label: "Projects", icon: "▣", page: projects },
  { id: "changelog", label: "Change Log", icon: "↻", page: changelog },
  { id: "journal", label: "Journal", icon: "◔", page: journal },
  { id: "worklog", label: "Work Log", icon: "◷", page: worklog },
  { id: "docs", label: "Documentation", icon: "▤", page: documents },
  { id: "portfolio", label: "Portfolio", icon: "◇", page: portfolio },
  { id: "fivewhy", label: "5-Why", icon: "✦", page: fivewhy },
  { id: "settings", label: "Settings", icon: "⚙", page: settings },
  { id: "help", label: "Help", icon: "?", page: help },
];

let currentPage = "dashboard";

function renderNav() {
  const nav = document.getElementById("nav");
  nav.innerHTML = PAGE_DEFS.map((p) => {
    const extra = p.badgeId ? `<span id="${p.badgeId}" class="nav-badge"></span>` : "";
    return `<button class="nav-item ${p.id === currentPage ? "active" : ""}" data-page="${p.id}">${p.icon} <span>${p.label}</span>${extra}</button>`;
  }).join("");
}

export function switchPage(id) {
  if (currentPage === id) refreshPage();
  currentPage = id;
  renderNav();
  renderPage(id);
}

async function refreshPage() {
  const p = PAGE_DEFS.find((p) => p.id === currentPage);
  if (p) {
    const el = document.getElementById("content");
    if (p.page.render) await p.page.render(el);
    updateBadge();
  }
}

async function renderPage(id) {
  const p = PAGE_DEFS.find((p) => p.id === id);
  if (!p) return;
  const el = document.getElementById("content");
  el.innerHTML = `<div class="page-max"><div class="empty"><div class="big">Loading…</div></div></div>`;
  try {
    if (p.page.render) await p.page.render(el);
  } catch (e) {
    el.innerHTML = `<div class="page-max"><div class="empty"><div class="big">Error loading ${id}</div><div class="small muted mt-2">${esc(String(e && e.message ? e.message : e))}</div></div></div>`;
  }
  updateBadge();
}

async function updateBadge() {
  try {
    const stats = await call("get_dashboard_stats");
    const el = document.getElementById("tasks-badge");
    if (el && stats) el.textContent = stats.openTasks || "";
  } catch {}
}

export async function loadCachedProjects() {
  return await loadProjects();
}

export async function loadCachedTree() {
  return await loadProjectTree();
}

const bus = {};
export function on(evt, fn) {
  (bus[evt] ||= []).push(fn);
}
export function go(evt, ...args) {
  (bus[evt] || []).forEach((fn) => fn(...args));
}

export function openModal(id, html) {
  const el = document.getElementById(id);
  if (!el) return;
  el.innerHTML = html;
  el.classList.remove("hidden");
  el.style.animation = "none";
  void el.offsetWidth;
  el.style.animation = "";
  const firstInput = el.querySelector("input,textarea,select");
  if (firstInput) setTimeout(() => firstInput.focus(), 60);
}

export function closeModal(id) {
  const el = document.getElementById(id);
  if (!el) return;
  el.classList.add("hidden");
  el.innerHTML = "";
}

function isTextualAttachment(a) {
  const m = (a.mime || "").toLowerCase();
  if (m.startsWith("text/")) return true;
  if (["application/json", "application/xml", "application/rtf", "application/javascript"]
      .some(x => m.startsWith(x))) return true;
  return /\.(txt|md|markdown|csv|tsv|log|json|xml|yaml|yml|ini|toml|html|htm|css|js|ts|py|rs|sql|sh|bat|java|c|cpp|h)$/i
    .test(a.filename || "");
}

function dataUrlToText(a, maxLen = 40000) {
  try {
    const i = (a.dataUrl || "").indexOf("base64,");
    if (i < 0) return "";
    const bin = atob(a.dataUrl.slice(i + 7));
    const bytes = new Uint8Array(bin.length);
    for (let k = 0; k < bin.length; k++) bytes[k] = bin.charCodeAt(k);
    let text = new TextDecoder("utf-8").decode(bytes);
    if (text.length > maxLen) text = text.slice(0, maxLen) + `\n\n… (truncated, ${a.size} bytes total)`;
    return text;
  } catch { return ""; }
}

async function saveAttachmentToDisk(a) {
  const api = (window.__TAURI__ || {}) && (window.__TAURI__.dialog || {});
  if (!api || typeof api.save !== "function") {
    toast("File save dialog is unavailable.", true);
    return;
  }
  const filters = a.mime ? [{ name: "File", extensions: [(a.filename.split(".").pop() || "bin")] }] : [];
  const dest = await api.save({ defaultPath: a.filename || "attachment", filters });
  if (!dest) return;
  try {
    await call("save_attachment", { id: a.id, dest_path: dest });
    toast(`Saved to ${dest}`);
  } catch (e) { toast(e.message, true); }
}

function openImageViewer(a) {
  const el = document.getElementById("image-viewer");
  if (!el || !a || !a.dataUrl) return;
  const saveBtn = `<div class="row mt-1"><button class="btn btn-sm" data-att-save="${a.id}">&#11015; Save as&hellip;</button></div>`;
  let body;
  if (a.isImage) {
    body = `<img src="${esc(a.dataUrl)}" alt="${esc(a.filename || "attachment")}">`;
  } else if (a.isTextual || isTextualAttachment(a)) {
    const text = dataUrlToText(a);
    body = text
      ? `<pre class="att-doc-preview">${esc(text)}</pre>`
      : `<div class="xs muted">No readable preview for this file.</div>`;
  } else {
    body = `<div class="xs muted">Binary file — use <b>Save as&hellip;</b> to download it.</div>`;
  }
  el.innerHTML = `<div class="lightbox" data-lightbox-close>
      <div class="lightbox-cap">${esc(a.filename || "")} · ${fmtBytes(a.size)}</div>
      <div class="att-view-body">${body}</div>
      ${saveBtn}
    </div>`;
  el.classList.remove("hidden");
  const save = el.querySelector("[data-att-save]");
  if (save) save.addEventListener("click", () => saveAttachmentToDisk(a));
}

function closeImageViewer() {
  closeModal("image-viewer");
}

export function showAttachment(a) {
  openImageViewer(a);
}

function bindImageViewerClicks(root) {
  root.querySelectorAll("[data-att-view], [data-att-prev]").forEach(el => {
    el.addEventListener("click", () => {
      const atts = window.__dtTaskAttachments || [];
      const found = atts.find(x => String(x.id) === el.dataset.attView || String(x.id) === el.dataset.attPrev);
      if (found) openImageViewer(found);
    });
  });
  root.querySelectorAll("[data-att-save]").forEach(el => {
    el.addEventListener("click", () => {
      const atts = window.__dtTaskAttachments || [];
      const found = atts.find(x => String(x.id) === el.dataset.attSave);
      if (found) saveAttachmentToDisk(found);
    });
  });
}

export function toast(message, isError = false) {
  const wrap = document.getElementById("toasts");
  const t = document.createElement("div");
  t.className = `toast ${isError ? "error" : ""}`;
  t.textContent = message;
  wrap.appendChild(t);
  setTimeout(() => {
    t.style.opacity = "0";
    t.style.transition = "opacity .3s";
    setTimeout(() => t.remove(), 300);
  }, 3800);
}

export function confirmDialog(
  message,
  { title = "Please confirm", confirmLabel = "Delete", danger = true } = {}
) {
  const el = document.getElementById("confirm-modal");
  if (!el) return Promise.resolve(false);
  return new Promise((resolve) => {
    const yesClass = danger ? "btn btn-danger" : "btn btn-primary";
    el.innerHTML = `<div class="modal" style="width:400px">
      <div class="modal-head"><h2>${esc(title)}</h2></div>
      <div class="modal-body"><p class="small" style="line-height:1.6">${esc(message)}</p></div>
      <div class="modal-foot">
        <button class="btn" id="confirm-no" type="button">Cancel</button>
        <button class="${yesClass}" id="confirm-yes" type="button">${esc(confirmLabel)}</button>
      </div>
    </div>`;
    el.classList.remove("hidden");
    const yes = document.getElementById("confirm-yes");
    const no = document.getElementById("confirm-no");
    no.focus();
    const done = (val) => {
      el.classList.add("hidden");
      el.innerHTML = "";
      yes.removeEventListener("click", onYes);
      no.removeEventListener("click", onNo);
      el.removeEventListener("click", onBackdrop);
      document.removeEventListener("keydown", onKey);
      resolve(val);
    };
    const onYes = () => done(true);
    const onNo = () => done(false);
    const onBackdrop = (e) => { if (e.target === el) done(false); };
    const onKey = (e) => { if (e.key === "Escape") done(false); };
    yes.addEventListener("click", onYes);
    no.addEventListener("click", onNo);
    el.addEventListener("click", onBackdrop);
    document.addEventListener("keydown", onKey);
  });
}

export function buildModal(id, head, body, foot = "") {
  return `<div class="modal" style="width:760px;max-width:94vw;max-height:90vh;overflow-y:auto">
    <div class="modal-head">${head}<button class="modal-close" data-close="${id}">&times;</button></div>
    <div class="modal-body">${body}</div>
    ${foot ? `<div class="modal-foot">${foot}</div>` : ""}
  </div>`;
}

export async function openNewTaskModal() {
  let projects = [];
  try { projects = await loadCachedProjects(); } catch {}
  const projectOpts = `<option value="">Select project…</option>${projects.map(p => `<option value="${p.id}">${esc(p.key)} — ${esc(p.name)}</option>`).join("")}`;
  openModal("task-modal", buildModal(
    "task-modal",
    `<h2>Create task</h2>`,
    `<div class="stack">
       <div class="field"><label>Project</label><select id="nt-project" class="select">${projectOpts}</select></div>
       <div class="field"><label>Title</label><input id="nt-title" class="input" placeholder="Task title…"></div>
       <div class="field"><label>Description</label><textarea id="nt-desc" class="textarea" rows="3"></textarea></div>
       <div class="grid-3">
         <div class="field"><label>Priority</label><select id="nt-priority" class="select">${PRIORITIES.map(p=>`<option${p==='Medium'?' selected':''}>${p}</option>`).join("")}</select></div>
         <div class="field"><label>Type</label><select id="nt-type" class="select">${TYPES.map(t=>`<option>${t}</option>`).join("")}</select></div>
         <div class="field"><label>Status</label><select id="nt-status" class="select">${STATUSES.map(s=>`<option>${s}</option>`).join("")}</select></div>
       </div>
       <div class="grid-3">
         <div class="field"><label>Due date</label><input id="nt-due" class="input" type="date"></div>
         <div class="field"><label>Assignee</label><input id="nt-assignee" class="input" value="${esc(window.__dtUser?.name || 'Local User')}"></div>
         <div class="field"><label>Estimate (minutes)</label><input id="nt-estimate" class="input" type="number" placeholder="e.g. 120"></div>
       </div>
     </div>`,
    `<button class="btn" data-close="task-modal">Cancel</button>
     <button class="btn btn-primary" id="nt-save">Create task</button>`
  ));
  document.getElementById("nt-save").onclick = async () => {
    const project_id = Number(document.getElementById("nt-project").value);
    const title = document.getElementById("nt-title").value.trim();
    if (!project_id || !title) { toast("Project and title are required.", true); return; }
    try {
      const task = await call("create_task", {
        input: {
          project_id,
          title,
          description: document.getElementById("nt-desc").value.trim() || null,
          priority: document.getElementById("nt-priority").value,
          task_type: document.getElementById("nt-type").value,
          status: document.getElementById("nt-status").value,
          due_date: document.getElementById("nt-due").value || null,
          assignee: document.getElementById("nt-assignee").value.trim() || null,
          estimated_minutes: Number(document.getElementById("nt-estimate").value) || null,
        },
      });
      closeModal("task-modal");
      toast(`Created ${task.key}`);
      go("task", task.id);
      refreshPage();
    } catch (e) {
      toast(e.message, true);
    }
  };
}

export async function openTaskDetail(id) {
  const b = await loadTaskBundle(id);
  if (!b || !b.task) return;
  renderTaskView(b);
}

/**
 * Refresh the task editor in place after a mutation.
 *
 * Editing actions used to call openTaskDetail(), which re-rendered the
 * read-only view and threw the user out of the form on every save, tag,
 * comment, relationship or attachment change. This reloads the bundle and
 * re-renders the editor so it stays open, while snapshotting and restoring
 * any fields the user has typed into but not yet saved.
 */
async function reloadTaskEditor(id) {
  const snapshot = snapshotEditorFields();
  const b = await loadTaskBundle(id);
  if (!b || !b.task) { closeModal("task-modal"); return; }
  renderTaskEditor(b);
  restoreEditorFields(snapshot);
}

function snapshotEditorFields() {
  const snap = {};
  document.querySelectorAll('#task-modal [id^="tm-"]').forEach(el => {
    if (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT") {
      snap[el.id] = { tag: el.tagName, value: el.value };
    }
  });
  return snap;
}

function restoreEditorFields(snap) {
  Object.entries(snap || {}).forEach(([id, saved]) => {
    const el = document.getElementById(id);
    if (!el || el.tagName !== saved.tag) return;
    if (el.tagName === "SELECT") {
      // Only restore if the option still exists (e.g. the project list loaded).
      if ([...el.options].some(o => o.value === saved.value)) el.value = saved.value;
    } else {
      el.value = saved.value;
    }
  });
}

async function loadTaskBundle(id) {
  let task, history = [], comments = [], rels = [], myTags = [], tags = [], allTasks = [];
  try {
    [task, history, comments, rels, myTags, tags] = await Promise.all([
      call("get_task", { id }),
      call("get_task_history", { task_id: id }),
      call("list_comments", { task_id: id }),
      call("list_relationships", { task_id: id }),
      call("list_task_tags", { task_id: id }),
      call("list_tags"),
    ]);
  } catch (e) { toast(e.message, true); return null; }

  let projects = [];
  try { projects = await loadCachedProjects(); } catch {}
  try { allTasks = await loadTasks({ include_subtasks: false }); } catch {}
  let attachments = [];
  try {
    const list = await call("list_attachments", { task_id: id });
    attachments = (await Promise.all(
      list.map(a => call("read_attachment_data", { id: a.id }).catch(() => null))
    )).filter(Boolean);
  } catch {}
  if (task && !Array.isArray(task.attachments)) task.attachments = attachments;
  return { task, history, comments, rels, myTags, tags, allTasks, projects, attachments };
}

function buildTimeline(history) {
  if (!history.length) return `<div class="empty"><div class="big">No history yet</div></div>`;
  return history.map(h => {
    let detail = "";
    if (h.action === "created" && h.newValue) detail = `Status: ${esc(h.newValue)}`;
    else if (h.action === "completed" || h.action === "cancelled" || h.action === "reopened") detail = `<span class="ghost-big">${esc(h.oldValue||"—")} → ${esc(h.newValue||"—")}</span>`;
    else if (h.field === "status") detail = `<span class="ghost-big">${esc(h.oldValue||"—")} → ${esc(h.newValue||"—")}</span>`;
    else if (h.field) detail = `<span class="ghost-big">${esc(h.field)}</span> ${esc(h.oldValue||"—")} → ${esc(h.newValue||"—")}`;
    else detail = `${esc(h.action)}`;
    return `<div class="stack mb-0"><div class="tl-time">${fmtTime(h.createdAt)}</div><div class="tl-action">${statusBadge(h.action)} ${esc(h.field || '')}</div><div class="tl-detail">${detail}</div></div>`;
  }).join("");
}

function relLabelOf(r) {
  const out = r.direction === "outgoing";
  const map = {
    blocks: out ? "blocks" : "blocked by",
    blocked_by: out ? "is blocked by" : "blocks",
    relates_to: "relates to",
    duplicates: "duplicates",
    depends_on: out ? "depends on" : "is required by",
  };
  return map[r.relType] || r.relType;
}

function attThumbHtml(a) {
  if (!a.dataUrl) return '<span class="xs muted">…</span>';
  if (a.isImage) {
    return `<img class="att-thumb" src="${esc(a.dataUrl)}" alt="${esc(a.filename)}" data-att-view="${a.id}" title="${esc(a.filename)}">`;
  }
  const icon = isTextualAttachment(a) ? "\u{1F4C4}" : "\u{1F5CB}\uFE0F";
  return `<div class="att-tile" data-att-prev="${a.id}" title="${esc(a.filename)}"><span>${icon}</span><span class="att-tile-ext">${esc(extOf(a.filename))}</span></div>`;
}

function extOf(filename) {
  const parts = (filename || "").split(".");
  return parts.length > 1 ? parts.pop().toUpperCase() : "FILE";
}

function attachmentPreviewList(task) {
  const atts = task.attachments || [];
  if (!atts.length) return `<div class="xs muted">No attachments yet</div>`;
  return atts.map(a =>
    `<div class="att-item" data-att-id="${a.id}"><div class="att-frame">${attThumbHtml(a)}</div>
      <div class="xs muted ellipsis" style="flex:1">${esc(a.filename)}</div>
      <span class="xs muted">${fmtBytes(a.size)}</span>
      <button class="btn btn-xs btn-danger" data-att-del="${a.id}" title="Remove">&times;</button></div>`
  ).join("");
}

function attachmentViewList(task) {
  const atts = task.attachments || [];
  if (!atts.length) return `<div class="xs muted">No attachments yet</div>`;
  return atts.map(a =>
    `<div class="att-item"><div class="att-frame">${attThumbHtml(a)}</div>
      <div class="xs muted ellipsis" style="flex:1">${esc(a.filename)}</div>
      <span class="xs muted">${fmtBytes(a.size)}</span></div>`
  ).join("");
}

function renderTaskView(b) {
  const { task, history, comments, rels, myTags, projects } = b;
  const project = (projects || []).find(p => p.id === task.projectId);
  window.__dtTaskAttachments = task.attachments || [];

  const timeline = buildTimeline(history);
  const tagChips = (myTags || []).map(t => `<span class="tag-chip">${esc(t.name)}</span>`).join("") || `<span class="xs muted">No tags</span>`;
  const commentList = (comments || []).map(c =>
    `<div class="stack mb-0" style="border-left:2px solid var(--line);padding-left:10px">
       <div class="spread"><span class="xs muted">${esc(c.userName || 'Unknown')} · ${fmtDay(c.createdAt)}</span></div>
       <div class="small" style="line-height:1.6">${renderTextWithMentions(c.content)}</div>
     </div>`
  ).join("") || `<div class="empty"><div class="big">No comments yet</div></div>`;
  const relList = (rels || []).map(r =>
    `<div class="rel-item">
       <span class="xs muted">${esc(relLabelOf(r))}</span>
       <span class="key grow" style="cursor:pointer" onclick="window.openTask(${r.taskId})">${esc(r.taskKey)}</span>
       <span class="small grow" style="flex:2;cursor:pointer" onclick="window.openTask(${r.taskId})">${esc(r.taskTitle)}</span>
     </div>`
  ).join("") || `<div class="empty"><div class="big">No relationships</div></div>`;

  const projectHtml = project ? `<span class="key">${esc(project.key)}</span> <span class="small">${esc(project.name)}</span>` : esc(String(task.projectId));
  const metaLine = (label, value) => `<div class="field"><label class="xs">${label}</label><div class="small">${value}</div></div>`;
  const due = task.dueDate ? fmtDay(task.dueDate) : "—";
  const estimate = task.estimatedMinutes != null ? fmtDur(task.estimatedMinutes) : "—";

  openModal("task-modal", buildModal(
    "task-modal",
    `<div><div class="key mb-1">${esc(task.key)}</div><h2>${esc(task.title)}</h2></div>`,
    `<div class="grid" style="grid-template-columns:1.8fr 1fr;gap:24px">
       <div class="stack">
         <div><div class="xs muted mb-1">Description</div><div class="small" style="line-height:1.7">${renderTextWithMentions(task.description || 'No description')}</div></div>
         <div><div class="xs muted mb-1">Tags</div><div class="row" style="gap:8px;flex-wrap:wrap">${tagChips}</div></div>
         <div><div class="xs muted mb-1">Relationships</div><div id="tm-rels">${relList}</div></div>
<div><div class="xs muted mb-1">Attachments</div><div id="tm-attachments">${attachmentViewList(task)}</div></div>
          <div><div class="xs muted mb-1">Comments</div><div id="tm-comments">${commentList}</div></div>
          <div><div class="xs muted mb-1">History</div><div class="timeline">${timeline}</div></div>
       </div>
       <div class="stack" style="gap:12px">
         ${metaLine("Status", statusBadge(task.status))}
         ${metaLine("Priority", esc(task.priority))}
         ${metaLine("Type", esc(task.taskType))}
         ${metaLine("Project", projectHtml)}
         ${metaLine("Assignee", esc(task.assignee || "—"))}
         ${metaLine("Due date", due)}
         ${metaLine("Estimate", estimate)}
         ${metaLine("Created", fmtDay(task.createdAt))}
         ${metaLine("Updated", fmtDay(task.updatedAt))}
         ${task.completedAt ? metaLine("Completed", fmtDay(task.completedAt)) : ""}
       </div>
     </div>`,
    `<button class="btn btn-primary" id="tm-edit">Edit</button>
     <button class="btn btn-danger" id="tm-del-view">Delete</button>`
  ));

  bindImageViewerClicks(document.getElementById("task-modal"));

  document.getElementById("tm-edit").onclick = () => renderTaskEditor(b);
  document.getElementById("tm-del-view").onclick = async () => {
    if (!(await confirmDialog(`Delete ${task.key} — ${task.title}?`, { title: "Delete task" }))) return;
    try {
      await call("delete_task", { id: task.id });
      closeModal("task-modal");
      toast(`Deleted ${task.key}`);
      refreshPage();
    } catch (e) { toast(e.message, true); }
  };
}

function renderTaskEditor(b) {
  const { task, history, comments, rels, myTags, tags, allTasks, projects } = b;
  const id = task.id;
  window.__dtTaskAttachments = task.attachments || [];

  const projectOpts = projects.map(p =>
    `<option value="${p.id}" ${p.id===task.projectId?'selected':''}>${esc(p.key)} — ${esc(p.name)}</option>`
  ).join("");

  const statusSel = STATUSES.map(s => `<option value="${s}" ${s===task.status?'selected':''}>${s}</option>`).join("");
  const prioSel = PRIORITIES.map(p => `<option value="${p}" ${p===task.priority?'selected':''}>${p}</option>`).join("");
  const typeSel = TYPES.map(t => `<option value="${t}" ${t===task.taskType?'selected':''}>${t}</option>`).join("");

  const timeline = buildTimeline(history);

  const myTagIds = (myTags || []).map(t => t.id);
  const tagChips = (myTags || []).map(t =>
    `<span class="tag-chip">${esc(t.name)}<button data-tag-del="${t.id}" title="Remove tag">&times;</button></span>`
  ).join("") || `<span class="xs muted">No tags</span>`;
  const tagOptions = (tags || []).filter(t => !myTagIds.includes(t.id))
    .map(t => `<option value="${esc(t.name)}"></option>`).join("");

  const commentList = (comments || []).map(c =>
    `<div class="stack mb-0" style="border-left:2px solid var(--line);padding-left:10px">
       <div class="spread"><span class="xs muted">${esc(c.userName || 'Unknown')} · ${fmtDay(c.createdAt)}</span>
         <button class="btn btn-sm btn-danger" data-comment-del="${c.id}" title="Delete comment">Delete</button></div>
       <div class="small" style="line-height:1.6">${renderTextWithMentions(c.content)}</div>
     </div>`
  ).join("") || `<div class="empty"><div class="big">No comments yet</div></div>`;

  const relList = (rels || []).map(r =>
    `<div class="rel-item">
       <span class="xs muted">${esc(relLabelOf(r))}</span>
       <span class="key grow" style="cursor:pointer" onclick="window.openTask(${r.taskId})">${esc(r.taskKey)}</span>
       <span class="small grow" style="flex:2;cursor:pointer" onclick="window.openTask(${r.taskId})">${esc(r.taskTitle)}</span>
       <button class="btn btn-sm btn-danger" data-rel-del="${r.id}">×</button>
     </div>`
  ).join("") || `<div class="empty"><div class="big">No relationships</div></div>`;

  const relTaskOpts = (allTasks || []).filter(t => (t.task ? t.task.id : t.id) !== id)
    .map(t => `<option value="${t.task ? t.task.id : t.id}">${esc(t.task ? `${t.task.key} — ${t.task.title}` : `${t.key} — ${t.title}`)}</option>`)
    .join("");

  openModal("task-modal", buildModal(
    "task-modal",
    `<div><div class="key mb-1">${esc(task.key)}</div><h2>${esc(task.title)}</h2></div>`,
    `<div class="grid" style="grid-template-columns:1.8fr 1fr;gap:24px">
       <div class="stack">
         <div><div class="xs muted mb-1">Description</div><div class="small" style="line-height:1.7">${renderTextWithMentions(task.description || 'No description')}</div></div>
         <div><div class="xs muted mb-1">Tags</div><div class="row" style="gap:8px;flex-wrap:wrap">${tagChips}</div>
           <div class="row mt-1"><input id="tm-tag-input" class="input" style="flex:1" list="tm-tag-list" placeholder="Add tag or create new…">
             <datalist id="tm-tag-list">${tagOptions}</datalist>
             <button class="btn btn-sm" id="tm-tag-add">Add</button></div></div>
         <div><div class="xs muted mb-1">Relationships</div>
           <div id="tm-rels">${relList}</div>
           <div class="row mt-1">
             <select id="tm-rel-type" class="select" style="flex:1.1">
               <option>blocks</option><option>blocked_by</option><option>relates_to</option><option>duplicates</option><option>depends_on</option>
             </select>
             <select id="tm-rel-task" class="select" style="flex:1.6"><option value="">Other task…</option>${relTaskOpts}</select>
              <button class="btn btn-sm" id="tm-rel-add">Link</button>
            </div></div>
          <div><div class="xs muted mb-1">Attachments</div>
            <div id="tm-attachments">${attachmentPreviewList(task)}</div>
            <div class="row mt-1"><input id="tm-attch-file" type="file" hidden>
              <button class="btn btn-sm" id="tm-attch-btn">&#128206; Attach files</button>
              <span class="xs muted ml-auto">or paste with Ctrl&#8984;+V</span></div>
          </div>
          <div><div class="xs muted mb-1">Comments</div><div id="tm-comments">${commentList}</div>
           <div class="row mt-1"><input id="tm-comment-input" class="input" style="flex:1" placeholder="Add a comment…">
             <button class="btn btn-sm" id="tm-comment-add">Post</button></div></div>
         <div><div class="xs muted mb-1">History</div><div class="timeline">${timeline}</div></div>
         <div><div class="xs muted mb-1">Quick work log</div>
           <textarea id="task-wl-text" class="textarea" rows="2" placeholder="What did you do on this task?"></textarea>
           <div class="row mt-1"><input id="task-wl-dur" class="input" style="width:120px" placeholder="minutes" type="number"><button class="btn btn-sm btn-primary ml-auto" id="task-wl-save">Log work</button></div>
         </div>
       </div>
       <div class="stack" style="gap:12px">
         <div class="field"><label class="xs">Status</label><select id="tm-status" class="select">${statusSel}</select></div>
         <div class="field"><label class="xs">Priority</label><select id="tm-priority" class="select">${prioSel}</select></div>
         <div class="field"><label class="xs">Type</label><select id="tm-type" class="select">${typeSel}</select></div>
         <div class="field"><label class="xs">Project</label><select id="tm-project" class="select">${projectOpts}</select></div>
         <div class="field"><label class="xs">Assignee</label><input id="tm-assignee" class="input" value="${esc(task.assignee || '')}"></div>
         <div class="field"><label class="xs">Due date</label><input id="tm-due" class="input" type="date" value="${task.dueDate || ''}"></div>
         <div class="field"><label class="xs">Estimate (minutes)</label><input id="tm-estimate" class="input" type="number" placeholder="${task.estimatedMinutes != null ? task.estimatedMinutes : 'e.g. 120'}" value="${task.estimatedMinutes ?? ''}"></div>
         <button class="btn" id="tm-timer">&#9654; Start timer</button>
         <button class="btn btn-primary" id="tm-save">Save changes</button>
         <button class="btn btn-danger" id="tm-delete">Delete task</button>
       </div>
     </div>`,
    ''
  ));

  bindImageViewerClicks(document.getElementById("task-modal"));

  document.getElementById("tm-save").onclick = async () => {
    try {
      await call("update_task", {
        id: task.id,
        input: {
          status: document.getElementById("tm-status").value,
          priority: document.getElementById("tm-priority").value,
          task_type: document.getElementById("tm-type").value,
          project_id: Number(document.getElementById("tm-project").value),
          assignee: document.getElementById("tm-assignee").value || null,
          due_date: document.getElementById("tm-due").value || null,
          estimated_minutes: Number(document.getElementById("tm-estimate").value) || null,
        },
      });
      toast(`Updated ${task.key}`);
      refreshPage();
      reloadTaskEditor(task.id);
    } catch (e) { toast(e.message, true); }
  };

  document.getElementById("tm-timer").onclick = async () => {
    try {
      const t = await call("start_timer", { input: { task_id: task.id, note: null } });
      toast(`Timer started on ${t.taskKey}`);
      refreshTimerWidget();
    } catch (e) { toast(e.message, true); }
  };

  document.getElementById("tm-delete").onclick = async () => {
    if (!(await confirmDialog(`Delete ${task.key} — ${task.title}?`, { title: "Delete task" }))) return;
    try {
      await call("delete_task", { id: task.id });
      closeModal("task-modal");
      toast(`Deleted ${task.key}`);
      refreshPage();
    } catch (e) { toast(e.message, true); }
  };

  document.getElementById("tm-tag-add").onclick = async () => {
    const name = document.getElementById("tm-tag-input").value.trim();
    if (!name) { toast("Enter a tag name.", true); return; }
    try {
      let tag = (tags || []).find(t => t.name.toLowerCase() === name.toLowerCase());
      let tagId;
      if (tag) {
        tagId = tag.id;
      } else {
        tag = await call("create_tag", { input: { name } });
        tagId = tag.id;
      }
      await call("add_task_tag", { task_id: id, tag_id: tagId });
      toast("Tag added");
      reloadTaskEditor(task.id);
    } catch (e) { toast(e.message, true); }
  };

  document.querySelectorAll("[data-tag-del]").forEach(el => {
    el.addEventListener("click", async () => {
      try {
        await call("remove_task_tag", { task_id: id, tag_id: Number(el.dataset.tagDel) });
        reloadTaskEditor(task.id);
      } catch (e) { toast(e.message, true); }
    });
  });

  document.getElementById("tm-comment-add").onclick = async () => {
    const content = document.getElementById("tm-comment-input").value.trim();
    if (!content) { toast("Comment is empty.", true); return; }
    try {
      await call("create_comment", { input: { task_id: id, content } });
      reloadTaskEditor(task.id);
    } catch (e) { toast(e.message, true); }
  };

  document.querySelectorAll("[data-comment-del]").forEach(el => {
    el.addEventListener("click", async () => {
      if (!(await confirmDialog("Delete this comment?", { title: "Delete comment" }))) return;
      try {
        await call("delete_comment", { id: Number(el.dataset.commentDel) });
        reloadTaskEditor(task.id);
      } catch (e) { toast(e.message, true); }
    });
  });

  document.getElementById("tm-rel-add").onclick = async () => {
    const relType = document.getElementById("tm-rel-type").value;
    const target = Number(document.getElementById("tm-rel-task").value);
    if (!target) { toast("Choose the other task.", true); return; }
    try {
      await call("add_relationship", { input: { source_task_id: task.id, target_task_id: target, rel_type: relType } });
      toast("Relationship added");
      reloadTaskEditor(task.id);
    } catch (e) { toast(e.message, true); }
  };

  document.querySelectorAll("[data-rel-del]").forEach(el => {
    el.addEventListener("click", async () => {
      if (!(await confirmDialog("Remove this relationship?", { title: "Remove relationship", confirmLabel: "Remove" }))) return;
      try {
        await call("remove_relationship", { id: Number(el.dataset.relDel) });
        reloadTaskEditor(task.id);
      } catch (e) { toast(e.message, true); }
    });
  });

  document.getElementById("task-wl-save").onclick = async () => {
    const text = document.getElementById("task-wl-text").value.trim();
    const dur = Number(document.getElementById("task-wl-dur").value) || 0;
    if (!text) { toast("Description required.", true); return; }
    try {
      await call("create_work_log", {
        input: { task_id: task.id, project_id: task.projectId, description: text, duration_minutes: dur || null },
      });
      toast("Work logged");
      reloadTaskEditor(task.id);
    } catch (e) { toast(e.message, true); }
  };

  const attachBtn = document.getElementById("tm-attch-btn");
  const attachInput = document.getElementById("tm-attch-file");
  if (attachBtn && attachInput) {
    attachBtn.onclick = () => attachInput.click();
    attachInput.onchange = async () => {
      const files = Array.from(attachInput.files || []);
      if (!files.length) return;
      attachInput.value = "";
      for (const f of files) {
        try {
          const buf = new Uint8Array(await f.arrayBuffer());
          let bin = "";
          const chunk = 0x8000;
          for (let i = 0; i < buf.length; i += chunk) {
            bin += String.fromCharCode(...buf.subarray(i, i + chunk));
          }
          const data = btoa(bin);
          await call("add_attachment_bytes", {
            input: { data, filename: f.name, task_id: id, project_id: null, mime: f.type || null },
          });
          toast(`Attached ${f.name}`);
        } catch (e) { toast(`Failed to attach ${f.name}: ${e.message}`, true); }
      }
      reloadTaskEditor(task.id);
    };
  }
  document.querySelectorAll("[data-att-del]").forEach(el => {
    el.addEventListener("click", async () => {
      if (!(await confirmDialog("Remove this attachment?", { title: "Remove attachment", confirmLabel: "Remove" }))) return;
      try {
        await call("remove_attachment", { id: Number(el.dataset.attDel) });
        reloadTaskEditor(task.id);
      } catch (e) { toast(e.message, true); }
    });
  });
}
export async function openProjectDetail(id) {
  let project, stats = {}, history = [], tags = [], myTags = [];
  try {
    project = await call("get_project", { id });
    [stats, history, tags, myTags] = await Promise.all([
      call("get_project_stats", { id }),
      call("get_project_history", { project_id: id, limit: 100 }),
      call("list_tags"),
      call("list_project_tags", { project_id: id }),
    ]);
  } catch (e) { toast(e.message, true); return; }

  let projects = [];
  try { projects = await loadCachedProjects(); } catch {}
  const projectOpts = `<option value="">None (top level)</option>${projects.filter(p=>p.id!==id).map(p =>
    `<option value="${p.id}" ${p.id===project.parentId?'selected':''}>${esc(p.key)} — ${esc(p.name)}</option>`
  ).join("")}`;

  const taskList = history.length ? history.slice(0, 20).map(h =>
    `<div class="row-item"><span class="key grow">${esc(h.taskKey)}</span><span class="grow" style="flex:2">${esc(h.taskTitle)}</span><span class="xs muted">${fmtDay(h.createdAt)}</span></div>`
  ).join("") : `<div class="empty"><div class="big">No recent activity</div></div>`;

  const myTagIds = (myTags || []).map(t => t.id);
  const projTagChips = (myTags || []).map(t =>
    `<span class="tag-chip">${esc(t.name)}<button data-proj-tag-del="${t.id}" title="Remove tag">&times;</button></span>`
  ).join("") || `<span class="xs muted">No tags</span>`;
  const projTagOptions = (tags || []).filter(t => !myTagIds.includes(t.id))
    .map(t => `<option value="${esc(t.name)}"></option>`).join("");

  openModal("project-modal", buildModal(
    "project-modal",
    `<h2>${esc(project.name)}</h2>`,
    `<div class="stack">
       <div class="stack"><div class="xs muted">Progress</div><div class="bar mt-1"><span style="width:${stats.progress||0}%"></span></div><div class="xs muted mt-1">${stats.total||0} tasks · ${Math.round(stats.progress||0)}% complete</div></div>
       <div class="grid-3">
         <div class="field"><label>Name</label><input id="pm-name" class="input" value="${esc(project.name)}"></div>
         <div class="field"><label>Key</label><input class="input" value="${esc(project.key)}" disabled></div>
         <div class="field"><label>Priority</label><select id="pm-priority" class="select">${PRIORITIES.map(p=>`<option${p===project.priority?' selected':''}>${p}</option>`).join("")}</select></div>
       </div>
       <div class="field"><label>Description</label><textarea id="pm-desc" class="textarea" rows="2">${esc(project.description)}</textarea></div>
       <div class="field"><label>Parent project</label><select id="pm-parent" class="select">${projectOpts}</select></div>
       <div><div class="xs muted mb-1">Tags</div><div class="row" style="gap:8px;flex-wrap:wrap">${projTagChips}</div>
         <div class="row mt-1"><input id="pm-tag-input" class="input" style="flex:1" list="pm-tag-list" placeholder="Add tag or create new…">
           <datalist id="pm-tag-list">${projTagOptions}</datalist>
           <button class="btn btn-sm" id="pm-tag-add">Add</button></div></div>
       <div class="spread mt-2"><button class="btn btn-primary" id="pm-save">Save changes</button><button class="btn btn-danger" id="pm-delete">Delete project</button></div>
       <div class="mt-3"><div class="xs muted mb-2">Recent history</div><div class="card card-pad" style="max-height:320px;overflow-y:auto">${taskList}</div></div>
     </div>`,
    ''
  ));

  document.getElementById("pm-save").onclick = async () => {
    try {
      await call("update_project", {
        id: project.id,
        input: {
          name: document.getElementById("pm-name").value.trim(),
          description: document.getElementById("pm-desc").value.trim(),
          priority: document.getElementById("pm-priority").value,
          parent_id: Number(document.getElementById("pm-parent").value) || null,
        },
      });
      toast(`Updated ${project.name}`);
      refreshPage();
      openProjectDetail(project.id);
    } catch (e) { toast(e.message, true); }
  };
  document.getElementById("pm-delete").onclick = async () => {
    if (!(await confirmDialog(`Delete ${project.name} and all its tasks?`, { title: "Delete project" }))) return;
    try {
      await call("delete_project", { id: project.id });
      closeModal("project-modal");
      toast(`Deleted ${project.name}`);
      refreshPage();
    } catch (e) { toast(e.message, true); }
  };

  document.getElementById("pm-tag-add").onclick = async () => {
    const name = document.getElementById("pm-tag-input").value.trim();
    if (!name) { toast("Enter a tag name.", true); return; }
    try {
      let tag = (tags || []).find(t => t.name.toLowerCase() === name.toLowerCase());
      let tagId;
      if (tag) {
        tagId = tag.id;
      } else {
        tag = await call("create_tag", { input: { name } });
        tagId = tag.id;
      }
      await call("add_project_tag", { project_id: id, tag_id: tagId });
      toast("Tag added");
      openProjectDetail(project.id);
    } catch (e) { toast(e.message, true); }
  };

  document.querySelectorAll("[data-proj-tag-del]").forEach(el => {
    el.addEventListener("click", async () => {
      try {
        await call("remove_project_tag", { project_id: id, tag_id: Number(el.dataset.projTagDel) });
        openProjectDetail(project.id);
      } catch (e) { toast(e.message, true); }
    });
  });
}

export async function openNewProjectModal() {
  let projects = [];
  try { projects = await loadCachedProjects(); } catch {}
  const parentOpts = `<option value="">None (top level)</option>${projects.map(p =>
    `<option value="${p.id}">${esc(p.key)} — ${esc(p.name)}</option>`
  ).join("")}`;
  openModal("project-modal", buildModal(
    "project-modal",
    `<h2>Create project</h2>`,
    `<div class="stack">
       <div class="field"><label>Parent project</label><select id="np-parent" class="select">${parentOpts}</select></div>
       <div class="grid-2"><div class="field"><label>Key</label><input id="np-key" class="input" placeholder="AGV"></div><div class="field"><label>Name</label><input id="np-name" class="input" placeholder="Project name"></div></div>
       <div class="field"><label>Description</label><textarea id="np-desc" class="textarea" rows="2"></textarea></div>
     </div>`,
    `<button class="btn" data-close="project-modal">Cancel</button>
     <button class="btn btn-primary" id="np-save">Create project</button>`
  ));
  document.getElementById("np-save").onclick = async () => {
    const key = document.getElementById("np-key").value.trim();
    const name = document.getElementById("np-name").value.trim();
    if (!key || !name) { toast("Key and name required.", true); return; }
    try {
      await call("create_project", {
        input: {
          key,
          name,
          description: document.getElementById("np-desc").value.trim() || null,
          parent_id: Number(document.getElementById("np-parent").value) || null,
        },
      });
      closeModal("project-modal");
      toast(`Created project ${key}`);
      refreshPage();
    } catch (e) { toast(e.message, true); }
  };
}

export async function openDocumentModal(id) {
  let doc;
  try {
    doc = await call("get_document", { id });
  } catch (e) { toast(e.message, true); return; }

  openModal("task-modal", buildModal(
    "task-modal",
    `<h2>${esc(doc.title)}</h2>`,
    `<div class="card card-pad doc-preview">${renderMarkdown(doc.content, { breaks: true }) || '<div class="empty"><div class="big">Empty document</div></div>'}</div>`,
    `<button class="btn" data-close="task-modal">Close</button>
     <button class="btn" id="doc-edit">Edit</button>
     <button class="btn btn-danger" id="doc-delete">Delete document</button>`
  ));

  document.getElementById("doc-delete").onclick = async () => {
    if (!(await confirmDialog(`Delete document "${doc.title}"?`, { title: "Delete document" }))) return;
    try {
      await call("delete_document", { id });
      closeModal("task-modal");
      toast("Document deleted");
      refreshPage();
    } catch (e) { toast(e.message, true); }
  };

  document.getElementById("doc-edit").onclick = async () => {
    openModal("task-modal", buildModal(
      "task-modal",
      `<h2>Edit document</h2>`,
      `<div class="stack">
         <div class="field"><label>Title</label><input id="doc-e-title" class="input" value="${esc(doc.title)}"></div>
         <div class="field"><label>Content</label><textarea id="doc-e-content" class="textarea" rows="10">${esc(doc.content)}</textarea></div>
       </div>`,
      `<button class="btn" data-close="task-modal">Cancel</button>
       <button class="btn btn-primary" id="doc-e-save">Save changes</button>`
    ));
    document.getElementById("doc-e-save").onclick = async () => {
      const title = document.getElementById("doc-e-title").value.trim();
      const content = document.getElementById("doc-e-content").value.trim();
      if (!title) { toast("Title required.", true); return; }
      try {
        await call("update_document", { id, input: { title, content } });
        closeModal("task-modal");
        toast("Document updated");
        refreshPage();
      } catch (e) { toast(e.message, true); }
    };
  };
}

let projectsCache = null;
async function getProjectsCached() {
  if (!projectsCache) projectsCache = await loadProjects();
  return projectsCache;
}

// search
const searchInput = document.getElementById("search-input");
const searchModal = document.getElementById("search-modal");
const searchQuery = document.getElementById("search-query");
const searchResults = document.getElementById("search-results");
let searchTimeout = null;

function openSearch() {
  searchModal.classList.remove("hidden");
  searchQuery.value = "";
  searchResults.innerHTML = `<div class="empty muted">Type to search projects, tasks, documents and work logs…</div>`;
  setTimeout(() => searchQuery.focus(), 40);
}

searchInput?.addEventListener("focus", openSearch);
searchInput?.addEventListener("click", openSearch);
document.addEventListener("keydown", (e) => {
  if (e.ctrlKey || e.metaKey) {
    if (e.key === "k") { e.preventDefault(); openSearch(); return; }
    if (e.key === "/") { e.preventDefault(); switchPage("help"); return; }
    if (e.key.toLowerCase() === "p") { e.preventDefault(); openPalette(); return; }
    const n = Number(e.key);
    if (n >= 1 && n <= PAGE_DEFS.length) { e.preventDefault(); switchPage(PAGE_DEFS[n - 1].id); return; }
  }
  if (e.key === "Escape") {
    ["task-modal", "project-modal", "settings-modal", "search-modal", "palette-modal", "image-viewer"].forEach((id) => {
      const el = document.getElementById(id);
      if (el && !el.classList.contains("hidden")) { el.classList.add("hidden"); el.innerHTML = ""; }
    });
  }
});

searchQuery?.addEventListener("input", () => {
  clearTimeout(searchTimeout);
  const q = searchQuery.value.trim();
  if (!q) { searchResults.innerHTML = `<div class="empty muted">Type to search projects, tasks, documents and work logs…</div>`; return; }
  searchTimeout = setTimeout(async () => {
    try {
      const groups = await call("global_search", { query: q });
      if (!groups.length) { searchResults.innerHTML = `<div class="empty">No results found.</div>`; return; }
      const catLabel = { task: "Tasks", project: "Projects", worklog: "Work Logs", document: "Documents", attachment: "Attachments" };
      searchResults.innerHTML = groups.map(g => {
        const items = g.items.map(i => {
          let action = "";
          if (g.category === "task") action = `onclick="window._openTask(${i.refId})"`;
          else if (g.category === "project") action = `onclick="window._openProject(${i.refId})"`;
          else if (g.category === "document") action = `onclick="window._openDocument(${i.refId})"`;
          return `<div class="row-item" ${action} style="cursor:pointer"><div><div class="small">${esc(i.title)}</div><div class="xs muted mt-1">${esc(i.body).substring(0,80)}</div></div></div>`;
        }).join("");
        return `<div class="mb-3"><div class="xs muted mb-1">${esc(catLabel[g.category] || g.category)}</div>${items}</div>`;
      }).join("");
    } catch (e) { searchResults.innerHTML = `<div class="empty">Search failed: ${esc(e.message)}</div>`; }
  }, 200);
});

searchModal?.addEventListener("click", (e) => {
  if (e.target.classList.contains("modal-backdrop")) {
    searchModal.classList.add("hidden");
    searchModal.innerHTML = "";
  }
});

document.addEventListener("click", (e) => {
  if (e.target.classList.contains("modal-backdrop")) e.target.classList.add("hidden");
  if (e.target.closest("[data-lightbox-close]")) closeImageViewer();
  const closer = e.target.closest("[data-close]");
  if (closer) {
    const id = closer.dataset.close;
    const el = document.getElementById(id);
    if (el) { el.classList.add("hidden"); el.innerHTML = ""; }
  }
});

window._openTask = openTaskDetail;
window._openProject = openProjectDetail;
window._openDocument = openDocumentModal;
window.openTask = openTaskDetail;
window.openProject = openProjectDetail;
window.openTaskByKey = openTaskByKey;

async function openTaskByKey(key) {
  try {
    const t = await call("get_task_by_key", { key });
    openTaskDetail(t.task ? t.task.id : t.id);
  } catch (e) {
    toast(e.message, true);
  }
}

// mention links (@@KEY)
document.addEventListener("click", (e) => {
  const m = e.target.closest("[data-task-key]");
  if (m) openTaskByKey(m.dataset.taskKey);
});

// --- command palette (Ctrl+P) ---
const paletteModal = document.getElementById("palette-modal");
let paletteItems = [];
let paletteIndex = 0;

function paletteActions() {
  const pageItems = PAGE_DEFS.map((p) => ({
    label: `Go to ${p.label}`,
    icon: p.icon,
    run: () => switchPage(p.id),
  }));
  const toolItems = [
    { label: "New task…", icon: "+", run: openNewTaskModal },
    { label: "New project…", icon: "▣", run: openNewProjectModal },
    { label: "Search everything (Ctrl+K)", icon: "⌕", run: openSearch },
    {
      label: "Create backup…", icon: "⛁",
      run: async () => {
        try { const path = await call("create_backup"); toast(`Backup created: ${path}`); }
        catch (e) { toast(e.message, true); }
      },
    },
    {
      label: "Export portfolio (MD + HTML)…", icon: "◇",
      run: async () => {
        try { const r = await call("export_portfolio"); toast(`Portfolio exported → ${r.markdownPath}`); }
        catch (e) { toast(e.message, true); }
      },
    },
  ];
  return [...pageItems, ...toolItems];
}

function openPalette() {
  if (!document.getElementById("palette-input")) restorePaletteMarkup();
  paletteModal.classList.remove("hidden");
  const inp = document.getElementById("palette-input");
  inp.value = "";
  renderPalette("");
  setTimeout(() => inp.focus(), 40);
}

function closePalette() {
  paletteModal.classList.add("hidden");
  paletteModal.innerHTML = "";
  restorePaletteMarkup();
}

function restorePaletteMarkup() {
  paletteModal.innerHTML = `<div class="modal" style="width:620px">
    <div class="modal-head">
      <h2>Quick actions</h2>
      <button class="modal-close" data-close="palette-modal">&times;</button>
    </div>
    <div class="modal-body">
      <input id="palette-input" class="input" placeholder="Type to filter actions, or a task key (e.g. AGV-001)..." autofocus>
      <div id="palette-results" class="mt-3"></div>
      <div class="xs muted mt-3">Enter runs the highlighted action · Esc closes</div>
    </div>
  </div>`;
  bindPaletteInputs();
}

function bindPaletteInputs() {
  const inp = document.getElementById("palette-input");
  if (!inp) return;
  inp.addEventListener("input", () => renderPalette(inp.value));
  inp.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") { e.preventDefault(); movePalette(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); movePalette(-1); }
    else if (e.key === "Enter") {
      const item = paletteItems[paletteIndex];
      if (item) { e.preventDefault(); item.run(); closePalette(); }
    }
  });
}

function movePalette(dir) {
  paletteIndex = Math.max(0, Math.min(paletteItems.length - 1, paletteIndex + dir));
  document.querySelectorAll("#palette-results .palette-item").forEach((el, i) => {
    el.classList.toggle("sel", i === paletteIndex);
  });
}

function renderPalette(q) {
  paletteIndex = 0;
  const ql = q.trim();
  let all = paletteActions();
  if (/^[A-Z0-9]+-\d+$/i.test(ql)) {
    all = [{ label: `Open task ${ql.toUpperCase()}`, icon: "✓", run: () => openTaskByKey(ql) }, ...all];
  }
  paletteItems = all.filter((a) => a.label.toLowerCase().includes(ql.toLowerCase())).slice(0, 12);
  document.getElementById("palette-results").innerHTML =
    paletteItems.map((a, i) =>
      `<div class="palette-item ${i === 0 ? 'sel' : ''}" data-i="${i}"><span class="pi-icon">${esc(a.icon)}</span><span>${esc(a.label)}</span></div>`
    ).join("") || `<div class="empty muted">No matching actions</div>`;
  document.querySelectorAll("#palette-results .palette-item").forEach((el) => {
    el.addEventListener("click", () => {
      const item = paletteItems[Number(el.dataset.i)];
      if (item) { item.run(); closePalette(); }
    });
  });
}

// --- header timer widget ---
export async function refreshTimerWidget() {
  const el = document.getElementById("timer-widget");
  if (!el) return;
  try {
    const t = await call("get_active_timer");
    if (t) {
      el.classList.remove("hidden");
      el.innerHTML = `<span class="tt-key">&#9678; ${esc(t.taskKey)}</span><span class="tt-time">${fmtDur(t.elapsedMinutes)}</span><button class="btn btn-sm btn-danger" id="timer-stop" title="Stop timer">Stop</button>`;
      el.querySelector("#timer-stop").onclick = async () => {
        try {
          const r = await call("stop_timer", { description: null });
          toast(`Logged ${fmtDur(r.elapsedMinutes)} on ${r.taskKey || "task"}`);
          refreshTimerWidget();
          refreshPage();
          updateBadge();
        } catch (e) { toast(e.message, true); }
      };
    } else {
      el.classList.add("hidden");
      el.innerHTML = "";
    }
  } catch {}
}

let timerPoll = null;
function startTimerPoll() {
  if (timerPoll) return;
  timerPoll = setInterval(refreshTimerWidget, 60000);
}

document.getElementById("btn-new-task")?.addEventListener("click", openNewTaskModal);
document.getElementById("btn-reload")?.addEventListener("click", () => {
  projectsCache = null;
  refreshPage();
  updateBadge();
});

document.getElementById("nav")?.addEventListener("click", (e) => {
  const btn = e.target.closest(".nav-item");
  if (btn && btn.dataset.page) switchPage(btn.dataset.page);
});

// custom titlebar: window controls + drag
async function setupTitlebar() {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    const win = getCurrentWindow();
    document.getElementById("win-min")?.addEventListener("click", () => win.minimize());
    document.getElementById("win-max")?.addEventListener("click", () => win.toggleMaximize());
    document.getElementById("win-close")?.addEventListener("click", () => win.close());
    const bar = document.getElementById("titlebar");
    bar?.addEventListener("mousedown", (e) => {
      if (e.button === 0 && !e.target.closest(".win-btn")) win.startDragging();
    });
    bar?.addEventListener("dblclick", (e) => {
      if (!e.target.closest(".win-btn")) win.toggleMaximize();
    });
  } catch {}
}

// listen to backend events → refresh
async function setupEvents() {
  try {
    const { listen } = window.__TAURI__.event;
    for (const evt of ["task-created", "task-updated", "task-deleted", "project-created", "project-updated", "project-deleted", "worklog-created", "timer-changed", "inbox-updated"]) {
      listen(evt, () => { refreshPage(); updateBadge(); refreshTimerWidget(); });
    }
  } catch {}
}

// init
document.addEventListener("contextmenu", (e) => e.preventDefault());
applyTheme();
renderNav();
initLogin();

let currentUser = null;

export function setUser(user) {
  currentUser = user;
  window.__dtUser = user || null;
  const avatar = document.getElementById("sidebar-avatar");
  const who = document.getElementById("sidebar-user");
  if (user) {
    who.textContent = user.name;
    loadSidebarAvatar(user);
  } else {
    avatar.innerHTML = "";
    avatar.textContent = "?";
    who.textContent = "…";
  }
}

async function loadSidebarAvatar(user) {
  const avatar = document.getElementById("sidebar-avatar");
  if (!avatar) return;
  let avatarData = "";
  try {
    avatarData = await call("get_setting", { key: `userAvatar:${user.id}` }) || "";
  } catch {}
  if (Number(window.__dtUser?.id) !== Number(user.id)) return;
  avatar.innerHTML = "";
  if (typeof avatarData === "string" && /^data:image\/(?:jpeg|jpg|png|webp);base64,/i.test(avatarData)) {
    const img = document.createElement("img");
    img.src = avatarData;
    img.alt = `${user.name} profile image`;
    avatar.appendChild(img);
  } else {
    avatar.textContent = initials(user.name);
  }
}

function initials(name) {
  return String(name || "?")
    .trim()
    .split(/\s+/)
    .map((w) => w[0] || "")
    .slice(0, 2)
    .join("")
    .toUpperCase();
}

function showLoginView(screen, view) {
  document.getElementById("login-view-form").classList.toggle("hidden", view !== "form");
  document.getElementById("login-view-setup").classList.toggle("hidden", view !== "setup");
}

function clearLoginErrors() {
  document.getElementById("login-error").textContent = "";
  document.getElementById("setup-error").textContent = "";
}

async function startApp(user) {
  setUser(user);
  document.getElementById("login-screen").classList.add("hidden");
  document.getElementById("login-screen").innerHTML = "";
  renderPage("dashboard");
  updateBadge();
  setupEvents();
  setupTitlebar();
  refreshTimerWidget();
  startTimerPoll();
  notifyOverdue();
}

function applyTheme() {
  document.body.classList.toggle("theme-light", localStorage.getItem("dt.theme") === "light");
}

async function notifyOverdue() {
  try {
    const overdue = await call("get_overdue_tasks");
    if (!overdue || !overdue.length) return;
    const title = `DuckTrack — ${overdue.length} overdue task${overdue.length === 1 ? "" : "s"}`;
    const body = overdue.slice(0, 4).map((t) => `${t.key}: ${t.title}`).join(" · ");
    try {
      await call("plugin:notification|notify", { options: { title, body } });
    } catch {
      toast(`⚠ ${title} — ${body}`);
    }
  } catch {}
}

async function initLogin() {
  const screen = document.getElementById("login-screen");
  if (window.__dtProbe) window.__dtProbe("initLogin START on this screen");
  let users = [];
  try {
    users = await call("list_users");
    if (window.__dtProbe) window.__dtProbe("initLogin list_users OK count=" + (users.length) + " " + JSON.stringify(users));
    document.title = "DuckTrack — " + users.length + " user(s)";
    try { await call("debug_log", { msg: "initLogin OK: list_users -> " + JSON.stringify(users) }); } catch {}
  } catch (e) {
    document.title = "DuckTrack — list_users ERROR: " + e.message;
    try { await call("debug_log", { msg: "initLogin ERROR: " + e.message + " | title='" + document.title + "'" }); } catch {}
    showLoginView(screen, "setup");
    document.getElementById("setup-name").disabled = true;
    document.getElementById("setup-password").disabled = true;
    document.getElementById("setup-error").textContent = e.message || "Could not load users.";
    return;
  }

  const onEnter = (ev) => {
    if (ev.key === "Enter") {
      if (!users.length) doSetup();
      else doLogin();
    }
  };

  if (!users.length) {
    document.getElementById("login-title").textContent = "Create your account";
    showLoginView(screen, "setup");
    document.getElementById("btn-setup").onclick = doSetup;
    document.getElementById("setup-name").addEventListener("keydown", onEnter);
    document.getElementById("setup-password").addEventListener("keydown", onEnter);
  } else {
    document.getElementById("login-title").textContent = "Sign in";
    const sel = document.getElementById("login-user");
    sel.innerHTML = users.map((u) => `<option value="${u.id}">${esc(u.name)}</option>`).join("");
    const lastUser = localStorage.getItem("dt.lastUser");
    if (lastUser) sel.value = lastUser;

    showLoginView(screen, "form");
    document.getElementById("btn-login").onclick = doLogin;
    document.getElementById("login-password").addEventListener("keydown", onEnter);
    document.getElementById("login-user").addEventListener("change", () => document.getElementById("login-password").focus());
    setTimeout(() => document.getElementById("login-password").focus(), 60);
  }
}

async function doSetup() {
  const name = document.getElementById("setup-name").value.trim();
  const password = document.getElementById("setup-password").value;
  clearLoginErrors();
  if (!name) { document.getElementById("setup-error").textContent = "Name is required."; return; }
  if (!password) { document.getElementById("setup-error").textContent = "Password is required."; return; }
  try {
    await call("create_user", { input: { name, password } });
    const user = await call("login_user", { username: name, password });
    try { await call("debug_log", { msg: `doLogin RESULT SUCCESS user_id=${user.id} name=${JSON.stringify(user.name)}` }); } catch {}
    localStorage.setItem("dt.lastUser", user.id);
    startApp(user);
  } catch (e) {
    document.getElementById("setup-error").textContent = e.message;
  }
}

async function doLogin() {
  const sel = document.getElementById("login-user");
  const name = sel.options[sel.selectedIndex]?.text || "";
  const password = document.getElementById("login-password").value;
  clearLoginErrors();
  if (!name || !password) { document.getElementById("login-error").textContent = "Enter your password."; return; }
  try {
    try { await call("debug_log", { msg: `doLogin SUBMIT username=${JSON.stringify(name)} password_len=${password.length} pw_first=${JSON.stringify(password.slice(0,2))} pw_last=${JSON.stringify(password.slice(-2))} pw_chars=${[...password].map(c=>c.charCodeAt(0)).join(",")}` }); } catch {}
    const user = await call("login_user", { username: name, password });
    try { await call("debug_log", { msg: `doLogin RESULT OK user_id=${user.id} name=${JSON.stringify(user.name)}` }); } catch {}
    localStorage.setItem("dt.lastUser", user.id);
    try { await call("debug_log", { msg: `doLogin NAVIGATING calling startApp now` }); } catch {}
    startApp(user);
    try { await call("debug_log", { msg: `startApp RETURNED user=${JSON.stringify(user.name)}` }); } catch {}
    return user;
  } catch (e) {
    document.getElementById("login-error").textContent = e.message;
    document.getElementById("login-password").value = "";
  }
}

document.getElementById("btn-logout")?.addEventListener("click", async () => {
  try { await call("logout_user"); } catch {}
  localStorage.removeItem("dt.lastUser");
  location.reload();
});