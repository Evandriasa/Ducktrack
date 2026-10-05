import { call, loadProjects } from "./api.js";
import { esc, fmtDay } from "./ui.js";

let docFilter = null;

export async function render(container) {
  let docs = [];
  let projects = [];
  try {
    [docs, projects] = await Promise.all([
      call("list_documents", { project_id: docFilter }),
      loadProjects(),
    ]);
  } catch {}

  const cards = (docs || []).map(d => {
    const doc = d.document || d;
    const title = d.document ? d.document.title : d.title;
    return `<div class="card card-pad" style="cursor:pointer" data-id="${doc.id}">
      <div class="key">▤</div>
      <div class="small" style="font-weight:600;margin-top:10px">${esc(title)}</div>
      <div class="xs muted mt-1" style="color:var(--muted)">${esc(d.projectName || '')} · updated ${fmtDay(doc.updatedAt)}</div>
    </div>`;
  }).join("");

  container.innerHTML = `
    <div class="page-max" style="max-width:1200px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Knowledge base</div>
          <h1 class="page-title">Documentation</h1>
          <p class="page-desc">Keep the knowledge with the project instead of in your head.</p>
        </div>
      </div>
      <div class="toolbar" style="border:1px solid var(--line);border-radius:10px;margin-bottom:20px;width:100%">
        <select id="doc-filter" class="select" style="width:260px">
          <option value="">All projects</option>
          ${projects.map(p => `<option value="${p.id}" ${docFilter === p.id ? "selected" : ""}>${esc(p.key)} — ${esc(p.name)}</option>`).join("")}
        </select>
        <button class="btn btn-primary ml-auto" id="doc-new">+ New document</button>
      </div>
      <div class="grid" style="grid-template-columns:repeat(3,1fr);gap:16px">${cards || '<div class="empty">No documents yet</div>'}</div>
    </div>`;

  document.getElementById("doc-filter")?.addEventListener("change", async (e) => {
    docFilter = e.target.value ? Number(e.target.value) : null;
    await render(container);
  });

  document.getElementById("doc-new")?.addEventListener("click", async () => {
    const html = `<div class="modal" style="width:640px;max-width:94vw">
      <div class="modal-head"><h2>New document</h2><button class="modal-close" data-close="task-modal">&times;</button></div>
      <div class="modal-body stack">
        <div class="field"><label>Project</label><select id="doc-new-project" class="select">${projects.map(p => `<option value="${p.id}">${esc(p.key)} — ${esc(p.name)}</option>`).join("")}</select></div>
        <div class="field"><label>Title</label><input id="doc-new-title" class="input" placeholder="Document title"></div>
        <div class="field"><label>Content (Markdown plain text)</label><textarea id="doc-new-content" class="textarea" rows="10" placeholder="Write knowledge here…"></textarea></div>
      </div>
      <div class="modal-foot"><button class="btn" data-close="task-modal">Cancel</button><button class="btn btn-primary" id="doc-new-save">Create</button></div>
    </div>`;
    const { openModal } = await import("./app.js");
    openModal("task-modal", html);
    document.getElementById("doc-new-save").addEventListener("click", async () => {
      const project_id = Number(document.getElementById("doc-new-project").value);
      const title = document.getElementById("doc-new-title").value.trim();
      const content = document.getElementById("doc-new-content").value.trim();
      if (!project_id || !title) return;
      try {
        await call("create_document", { input: { project_id, title, content } });
        const { closeModal, toast } = await import("./app.js");
        closeModal("task-modal");
        toast("Document created");
        await render(container);
      } catch (e) {
        const { toast } = await import("./app.js");
        toast(e.message, true);
      }
    });
  });

  document.querySelectorAll(".card[data-id]").forEach(el => {
    el.addEventListener("click", async () => {
      const id = Number(el.dataset.id);
      const { openDocumentModal } = await import("./app.js");
      openDocumentModal(id);
    });
  });
}