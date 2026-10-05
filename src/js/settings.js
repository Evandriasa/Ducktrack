import { call } from "./api.js";
import { esc } from "./ui.js";

export async function render(container) {
  const workspace = await call("get_workspace_info").catch(() => null);
  const theme = localStorage.getItem("dt.theme") || "dark";
  let autoBackup = true;
  let retention = 30;
  try {
    const ab = await call("get_setting", { key: "autoBackup" });
    autoBackup = ab !== "0";
  } catch {}
  try {
    const r = await call("get_setting", { key: "backupRetentionCount" });
    if (r) retention = Number(r) || 30;
  } catch {}

  container.innerHTML = `
    <div class="page-max" style="max-width:900px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Configuration</div>
          <h1 class="page-title">Settings</h1>
          <p class="page-desc">Backup, restore, export and import your workspace.</p>
        </div>
      </div>
      <div class="stack" style="gap:20px">
        <div class="card card-pad">
          <div class="row">
            <div>
              <h3 class="small" style="font-weight:600">Users</h3>
              <p class="xs muted mt-1">Each user gets their own projects, tasks and content.</p>
            </div>
            <button class="btn btn-primary ml-auto" id="set-user-add">Add user</button>
          </div>
          <div id="set-users" class="mt-3"></div>
        </div>
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Workspace</h3>
          <p class="xs muted mt-1">${workspace ? "Location: " + esc(workspace) : 'Everything is stored locally.'}</p>
        </div>
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Appearance</h3>
          <p class="xs muted mt-1">Pick a color theme. Stored on this device.</p>
          <div class="row mt-3" style="gap:8px">
            <button class="btn ${theme === 'light' ? 'btn-primary' : ''}" id="set-theme-light">Light</button>
            <button class="btn ${theme === 'dark' ? 'btn-primary' : ''}" id="set-theme-dark">Dark</button>
          </div>
        </div>
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Tags</h3>
          <p class="xs muted mt-1">Rename or delete labels used across tasks and projects.</p>
          <div id="set-tags" class="mt-3"></div>
        </div>
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Backup</h3>
          <p class="xs muted mt-1">Create a snapshot of the database, or restore from an earlier one.</p>
          <div class="stack mt-3" style="gap:12px">
            <label class="setting-row" style="cursor:pointer">
              <input type="checkbox" id="set-autobackup" ${autoBackup ? 'checked' : ''}>
              <span class="grow">Keep automatic daily backups</span>
            </label>
            <div class="setting-row">
              <span class="grow">Keep the last</span>
              <input id="set-retention" class="input" type="number" min="1" value="${retention}" style="width:80px">
              <span>backups</span>
            </div>
            <div class="row">
              <button class="btn btn-primary" id="set-backup">Back up now</button>
              <button class="btn" id="set-prune">Prune to retention</button>
              <button class="btn" id="set-restore">Restore latest</button>
            </div>
          </div>
          <div id="set-backups" class="mt-3"></div>
        </div>
        <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Export / Import</h3>
          <p class="xs muted mt-1">Export copies the database and attachments. Import restores a full workspace.</p>
          <div class="row mt-3">
            <button class="btn" id="set-export">Export workspace</button>
            <input id="set-import-path" class="input" style="max-width:340px" placeholder="/path/to/export/dir">
            <button class="btn" id="set-import">Import</button>
          </div>
        </div>
      <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Updates</h3>
          <p class="xs muted mt-1">Check GitHub for a newer DuckTrack release.</p>
          <div class="row mt-3" style="gap:8px">
            <button class="btn btn-primary" id="set-check-updates">Check for updates</button>
            <button class="btn" id="set-open-github">Open GitHub</button>
            <button class="btn" id="set-open-releases">Releases</button>
          </div>
          <div id="set-update-status" class="mt-3"></div>
        </div>
      <div class="card card-pad">
          <h3 class="small" style="font-weight:600">Danger zone</h3>
          <p class="xs muted mt-1">Delete all projects, tasks, documents, work logs and history. The workspace is left completely empty.</p>
          <div class="row mt-3">
            <button class="btn btn-danger" id="set-reset">Reset workspace</button>
          </div>
        </div>
      </div>
    </div>`;

  async function refreshBackups() {
    const el = document.getElementById("set-backups");
    if (!el) return;
    el.innerHTML = `<div class="xs muted">Loading backups…</div>`;
    try {
      const backups = await call("list_backups");
      el.innerHTML = backups.length
        ? `<div class="xs muted mb-2">Available backups</div>` + backups.slice(0, 10).map(b =>
            `<div class="row-item"><span class="mono xs" style="color:var(--accent)">${esc(b)}</span></div>`).join("")
        : `<div class="empty" style="padding:12px"><div class="big">No backups yet</div></div>`;
    } catch (e) {
      el.innerHTML = `<div class="xs" style="color:var(--red)">${esc(e.message)}</div>`;
    }
  }
  refreshBackups();

  const { toast, confirmDialog } = await import("./app.js");

  async function refreshUsers() {
    const el = document.getElementById("set-users");
    if (!el) return;
    el.innerHTML = `<div class="xs muted">Loading users…</div>`;
    try {
      const users = await call("list_users");
      const avatars = new Map(await Promise.all(users.map(async user => [
        Number(user.id),
        await getUserAvatar(user.id)
      ])));
      el.innerHTML = users.length
        ? users.map(u => `
          <div class="row-item" style="cursor:default">
            <div class="avatar" data-user-avatar="${u.id}" style="width:28px;height:28px;font-size:11px"></div>
            <span class="grow">${esc(u.name)}</span>
            <button class="btn btn-sm" data-user-edit="${u.id}">Edit</button>
            <button class="btn btn-sm btn-danger" data-user-del="${u.id}">Delete</button>
          </div>`).join("")
        : `<div class="empty" style="padding:12px"><div class="big">No users</div></div>`;
      users.forEach(user => {
        const avatar = el.querySelector(`[data-user-avatar="${user.id}"]`);
        renderAvatarElement(avatar, user.name, avatars.get(Number(user.id)));
      });
      el.querySelectorAll("[data-user-edit]").forEach(b => b.addEventListener("click", () => editUser(Number(b.dataset.userEdit))));
      el.querySelectorAll("[data-user-del]").forEach(b => b.addEventListener("click", () => deleteUser(Number(b.dataset.userDel))));
    } catch (e) {
      el.innerHTML = `<div class="xs" style="color:var(--red)">${esc(e.message)}</div>`;
    }
  }
  refreshUsers();

  function initials(name) {
    return String(name || "?").trim().split(/\s+/).map(w => w[0] || "").slice(0, 2).join("").toUpperCase();
  }

  function avatarKey(userId) {
    return `userAvatar:${userId}`;
  }

  function isAvatarData(value) {
    return typeof value === "string" && /^data:image\/(?:jpeg|jpg|png|webp);base64,/i.test(value);
  }

  async function getUserAvatar(userId) {
    try {
      const value = await call("get_setting", { key: avatarKey(userId) });
      return isAvatarData(value) ? value : "";
    } catch {
      return "";
    }
  }

  async function saveUserAvatar(userId, value) {
    await call("set_setting", { key: avatarKey(userId), value: value || "" });
  }

  function renderAvatarElement(element, name, avatarData) {
    if (!element) return;
    element.innerHTML = "";
    if (isAvatarData(avatarData)) {
      const img = document.createElement("img");
      img.src = avatarData;
      img.alt = `${name || "User"} profile image`;
      element.appendChild(img);
    } else {
      element.textContent = initials(name);
    }
  }

  async function prepareAvatar(file) {
    if (!file) return "";
    const allowedTypes = ["image/jpeg", "image/png", "image/webp"];
    if (!allowedTypes.includes(file.type)) {
      throw new Error("Please choose a PNG, JPG or WebP image.");
    }
    if (file.size > 8 * 1024 * 1024) {
      throw new Error("Profile image must be smaller than 8 MB.");
    }
    const dataUrl = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result);
      reader.onerror = () => reject(new Error("Could not read the image."));
      reader.readAsDataURL(file);
    });
    const image = await new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error("Could not load the image."));
      img.src = dataUrl;
    });
    const size = 256;
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d");
    const sourceSize = Math.min(image.naturalWidth, image.naturalHeight);
    const sx = (image.naturalWidth - sourceSize) / 2;
    const sy = (image.naturalHeight - sourceSize) / 2;
    ctx.drawImage(image, sx, sy, sourceSize, sourceSize, 0, 0, size, size);
    return canvas.toDataURL("image/jpeg", 0.88);
  }

  document.getElementById("set-user-add")?.addEventListener("click", () => userModal(null));

  async function editUser(id) {
    try {
      const users = await call("list_users");
      const user = users.find(u => Number(u.id) === Number(id));
      if (!user) { toast("User could not be found.", true); return; }
      await userModal(user);
    } catch (e) {
      toast(e.message || "Could not load user.", true);
    }
  }

  async function userModal(user) {
    const { openModal, closeModal, buildModal, setUser } = await import("./app.js");

    const isEdit = !!user;
    const title = isEdit ? "Edit user" : "Add user";

    let avatarData = isEdit ? await getUserAvatar(user.id) : "";
    let avatarChanged = false;

    openModal("settings-modal", buildModal(
      "settings-modal",
      `<h2>${title}</h2>`,
      `<div class="stack">
         <div class="field">
           <label>Profile image</label>
           <div class="row" style="align-items:center;gap:16px">
             <div id="u-avatar-preview" class="avatar" style="width:72px;height:72px;font-size:20px;flex-shrink:0"></div>
             <div class="stack" style="gap:8px;flex:1">
               <input id="u-avatar-file" class="input" type="file" accept="image/jpeg,image/png,image/webp">
               <div class="xs muted">JPG, PNG or WebP. The image will be cropped and resized automatically.</div>
               <button id="u-avatar-remove" class="btn btn-sm" type="button" ${avatarData ? "" : "disabled"}>Remove image</button>
             </div>
           </div>
         </div>
         <div class="field"><label>Name</label><input id="u-name" class="input" value="${isEdit ? esc(user.name) : ""}" placeholder="e.g. Ada"></div>
         <div class="field"><label>${isEdit ? "New password (leave blank to keep)" : "Password"}</label><input id="u-pass" type="password" class="input" placeholder="Password"></div>
       </div>`,
      `<button class="btn" data-close="settings-modal">Cancel</button>
       <button class="btn btn-primary" id="u-save">${isEdit ? "Save changes" : "Create user"}</button>`
    ));

    const nameInput = document.getElementById("u-name");
    const avatarInput = document.getElementById("u-avatar-file");
    const avatarPreview = document.getElementById("u-avatar-preview");
    const avatarRemove = document.getElementById("u-avatar-remove");

    function updatePreview() {
      renderAvatarElement(avatarPreview, (nameInput.value || "").trim() || "?", avatarData);
      avatarRemove.disabled = !avatarData;
    }
    updatePreview();
    nameInput.addEventListener("input", updatePreview);

    avatarInput.addEventListener("change", async () => {
      const file = avatarInput.files?.[0];
      if (!file) return;
      try {
        avatarData = await prepareAvatar(file);
        avatarChanged = true;
        updatePreview();
      } catch (e) {
        toast(e.message || "Could not load image.", true);
        avatarInput.value = "";
      }
    });

    avatarRemove.addEventListener("click", () => {
      avatarData = "";
      avatarChanged = true;
      avatarInput.value = "";
      updatePreview();
    });

    document.getElementById("u-save").addEventListener("click", async () => {
      const name = nameInput.value.trim();
      const password = document.getElementById("u-pass").value;
      if (!name) { toast("Name is required.", true); return; }
      try {
        if (isEdit) {
          const input = { name };
          if (password) input.password = password;
          await call("update_user", { id: user.id, input });
          if (avatarChanged) await saveUserAvatar(user.id, avatarData);
          if (window.__dtUser && Number(window.__dtUser.id) === Number(user.id)) {
            setUser({ ...window.__dtUser, name });
          }
          toast("User updated");
        } else {
          if (!password) { toast("Password is required.", true); return; }
          const created = await call("create_user", { input: { name, password } });
          let newUserId = Number(created?.id ?? created);
          if (!Number.isFinite(newUserId) || newUserId <= 0) {
            const users = await call("list_users");
            const createdUser = users.find(u => u.name === name);
            newUserId = Number(createdUser?.id);
          }
          if (newUserId && avatarData) await saveUserAvatar(newUserId, avatarData);
          toast("User created");
        }
        closeModal("settings-modal");
        refreshUsers();
      } catch (e) { toast(e.message, true); }
    });
  }

  async function deleteUser(id) {
    if (!(await confirmDialog("Delete this user? They must have no projects of their own.", { title: "Delete user" }))) return;
    try {
      await call("delete_user", { id });
      toast("User deleted");
      refreshUsers();
    } catch (e) { toast(e.message, true); }
  }

  document.getElementById("set-backup")?.addEventListener("click", async () => {
    try {
      const path = await call("create_backup");
      toast(`Backup created: ${path}`);
      refreshBackups();
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-restore")?.addEventListener("click", async () => {
    if (!(await confirmDialog("Restore the latest backup? Current data will be replaced.", { title: "Restore backup", confirmLabel: "Restore", danger: false }))) return;
    try {
      await call("restore_backup", { name: null });
      toast("Workspace restored");
      refreshBackups();
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-export")?.addEventListener("click", async () => {
    try {
      const path = await call("export_workspace");
      toast(`Exported to ${path}`);
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-import")?.addEventListener("click", async () => {
    const path = document.getElementById("set-import-path").value.trim();
    if (!path) { toast("Enter the path to a DuckTrack export.", true); return; }
    if (!(await confirmDialog("Import will replace current data. Continue?", { title: "Import workspace", confirmLabel: "Import", danger: false }))) return;
    try {
      await call("import_workspace", { source_dir: path });
      toast("Workspace imported");
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-open-github")?.addEventListener("click", async () => {
    try {
      await call("open_github_page", { page: "repo" });
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-open-releases")?.addEventListener("click", async () => {
    try {
      await call("open_github_page", { page: "releases" });
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-check-updates")?.addEventListener("click", async () => {
    const status = document.getElementById("set-update-status");
    const btn = document.getElementById("set-check-updates");
    if (btn) { btn.disabled = true; btn.textContent = "Checking…"; }
    if (status) status.innerHTML = `<div class="xs muted">Contacting GitHub…</div>`;
    try {
      const info = await call("check_for_updates");
      if (!status) return;
      const head = info.updateAvailable
        ? `<div class="xs" style="color:var(--green)"><strong>Update available</strong> — ${esc(info.message)}</div>`
        : `<div class="xs muted">${esc(info.message)}</div>`;
      const meta = info.latest
        ? `<div class="xs muted mt-1">Latest release: ${esc(info.latest)}${info.publishedAt ? " · " + esc(String(info.publishedAt).slice(0, 10)) : ""}</div>`
        : "";
      const notes = info.notes
        ? `<div class="xs muted mt-2" style="white-space:pre-wrap">${esc(info.notes)}</div>`
        : "";
      const actions = `<div class="row mt-2" style="gap:8px">
          ${info.updateAvailable && info.releaseUrl ? `<button class="btn btn-sm btn-primary" id="set-update-open">Open release page</button>` : ""}
          <button class="btn btn-sm" id="set-update-repo">Open repository</button>
        </div>`;
      status.innerHTML = head + meta + notes + actions;

      document.getElementById("set-update-open")?.addEventListener("click", async () => {
        try {
          await call("open_github_page", { page: "releases" });
        } catch (e) { toast(e.message, true); }
      });
      document.getElementById("set-update-repo")?.addEventListener("click", async () => {
        try {
          await call("open_github_page", { page: "repo" });
        } catch (e) { toast(e.message, true); }
      });
    } catch (e) {
      if (status) status.innerHTML = `<div class="xs" style="color:var(--red)">${esc(e.message)}</div>`;
    } finally {
      if (btn) { btn.disabled = false; btn.textContent = "Check for updates"; }
    }
  });

  document.getElementById("set-reset")?.addEventListener("click", async () => {
    if (!(await confirmDialog("Reset the workspace? This deletes ALL projects, tasks, documents, work logs and history. This cannot be undone.", { title: "Reset workspace", confirmLabel: "Reset" }))) return;
    try {
      await call("reset_workspace");
      toast("Workspace reset");
    } catch (e) { toast(e.message, true); }
  });

  document.getElementById("set-theme-light")?.addEventListener("click", () => setTheme("light"));
  document.getElementById("set-theme-dark")?.addEventListener("click", () => setTheme("dark"));

  function setTheme(t) {
    localStorage.setItem("dt.theme", t);
    document.body.classList.toggle("theme-light", t === "light");
    document.querySelectorAll("#set-theme-light, #set-theme-dark").forEach(b => {
      b.classList.toggle("btn-primary", b.id === (t === "light" ? "set-theme-light" : "set-theme-dark"));
    });
  }

  async function refreshTags() {
    const el = document.getElementById("set-tags");
    if (!el) return;
    el.innerHTML = `<div class="xs muted">Loading tags…</div>`;
    try {
      const tags = await call("list_tags");
      el.innerHTML = tags.length
        ? tags.map(t => `
          <div class="tag-row">
            <span class="tag-chip">${esc(t.name)}</span>
            <span class="tag-count grow">${t.taskCount} task${t.taskCount === 1 ? "" : "s"} · ${t.projectCount} project${t.projectCount === 1 ? "" : "s"}</span>
            <input class="input" style="max-width:160px" id="tag-rename-${t.id}" placeholder="New name" value="${esc(t.name)}">
            <button class="btn btn-sm" data-tag-rename="${t.id}">Rename</button>
            <button class="btn btn-sm btn-danger" data-tag-del="${t.id}">Delete</button>
          </div>`).join("")
        : `<div class="xs muted">No tags yet.</div>`;
      el.querySelectorAll("[data-tag-rename]").forEach(b => b.addEventListener("click", async () => {
        const name = document.getElementById("tag-rename-" + b.dataset.tagRename)?.value.trim();
        if (!name) { toast("Tag name required.", true); return; }
        try {
          await call("update_tag", { id: Number(b.dataset.tagRename), input: { name } });
          toast("Tag renamed");
          refreshTags();
        } catch (e) { toast(e.message, true); }
      }));
      el.querySelectorAll("[data-tag-del]").forEach(b => b.addEventListener("click", async () => {
        if (!(await confirmDialog(`Delete tag? It is removed from all tasks and projects.`, { title: "Delete tag" }))) return;
        try {
          await call("delete_tag", { id: Number(b.dataset.tagDel) });
          toast("Tag deleted");
          refreshTags();
        } catch (e) { toast(e.message, true); }
      }));
    } catch (e) {
      el.innerHTML = `<div class="xs" style="color:var(--red)">${esc(e.message)}</div>`;
    }
  }
  refreshTags();

  const saveSetting = async (key, value) => {
    try { await call("set_setting", { key, value }); } catch (e) { toast(e.message, true); }
  };

  document.getElementById("set-autobackup")?.addEventListener("change", (e) => {
    saveSetting("autoBackup", e.target.checked ? "1" : "0");
  });

  document.getElementById("set-retention")?.addEventListener("change", (e) => {
    const v = Math.max(1, Number(e.target.value) || 30);
    e.target.value = v;
    saveSetting("backupRetentionCount", String(v));
  });

  document.getElementById("set-prune")?.addEventListener("click", async () => {
    try {
      await call("prune_backups", { keep: Math.max(1, Number(document.getElementById("set-retention")?.value) || 30) });
      toast("Backups pruned");
      refreshBackups();
    } catch (e) { toast(e.message, true); }
  });
}