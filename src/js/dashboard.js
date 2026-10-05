import { call } from "./api.js";
import { esc, fmtDay, fmtDur, dot, statusBadge } from "./ui.js";

export async function render(container) {
  let [stats, tasks, history, tree, overdue] = await Promise.all([
    call("get_dashboard_stats"),
    call("list_tasks", { filter: { status: "In Progress", include_subtasks: false } }),
    call("get_global_history", { filter: { limit: 12 } }),
    call("get_project_tree"),
    call("get_overdue_tasks").catch(() => []),
  ]);

  const today = new Date();
  const dateStr = today.toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });

  const hour = today.getHours();
  const period = hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";

  const first = String((window.__dtUser && window.__dtUser.name) || "friend")
    .trim()
    .split(/\s+/)[0] || "friend";

  const greeting = `${period}, ${esc(first)}.`;

  const activeTasks = (tasks || []).slice(0, 3);

  const recentActivity = (history || []).slice(0, 6).map((h) => {
    const day = fmtDay(h.createdAt);

    const color =
      h.action === "completed"
        ? "st-done"
        : h.action === "created"
          ? "st-progress"
          : h.action === "cancelled"
            ? "st-blocked"
            : "st-todo";

    let desc = "";

    if (h.action === "created") {
      desc = "created";
    } else if (h.action === "completed") {
      desc = "completed";
    } else if (h.action === "cancelled") {
      desc = "cancelled";
    } else if (h.action === "reopened") {
      desc = "reopened";
    } else if (h.field === "status") {
      desc = `${h.oldValue || "—"} → ${h.newValue || "—"}`;
    } else if (h.field) {
      desc = `${h.field} changed`;
    } else {
      desc = h.action;
    }

    return `
      <div class="row-item dashboard-change-row">

        <span class="xs muted dashboard-change-date">
          ${esc(day)}
        </span>

        <span class="dot ${color}"></span>

        <div class="grow">

          <div class="small">
            <span class="key">${esc(h.taskKey)}</span>
            <span> — ${esc(h.taskTitle || desc)}</span>
          </div>

          <div class="xs muted mt-1">
            ${esc(h.projectKey || "")}
            ${h.projectKey ? " · " : ""}
            ${esc(desc)}
          </div>

        </div>

      </div>`;
  }).join("");



  function* walk(nodes) {
    for (const n of nodes || []) {
      yield n;

      if (n.children && n.children.length) {
        yield* walk(n.children);
      }
    }
  }



  const allProjects = Array.from(walk(tree));



  for (const node of allProjects) {
    try {
      const s = await call("get_project_stats", {
        id: node.id,
      });

      node._statsProgress = s.progress;

    } catch {
      node._statsProgress = 0;
    }
  }



  const projectBarsReal = allProjects
    .slice(0, 4)
    .map((p) => {

      const pct = p._statsProgress || 0;

      return `
        <div>

          <div class="spread mb-1">

            <span class="small">
              ${esc(p.name)}
            </span>

            <span class="xs muted">
              ${Math.round(pct)}%
            </span>

          </div>

          <div class="bar">
            <span style="width:${pct}%"></span>
          </div>

        </div>`;
    })
    .join("");



  const projectOptions = allProjects
    .map((p) => `
      <option value="${p.id}">
        ${esc(p.name)}
      </option>
    `)
    .join("");



  const overdueRows = (overdue || [])
    .map((t) => `
      <div
        class="row-item"
        onclick="window.openTask(${t.id})"
      >

        <span class="overdue-dot"></span>

        <span
          class="key"
          style="width:90px"
        >
          ${esc(t.key)}
        </span>

        <span class="small grow">
          ${esc(t.title)}
        </span>

        <span class="xs muted">
          due ${esc(t.dueDate)}
        </span>

      </div>
    `)
    .join("");



  container.innerHTML = `

    <div class="page-max dashboard-page">


      <!-- =====================================================
           DASHBOARD HEADER
           ===================================================== -->

      <div class="page-head">

        <div>

          <div class="eyebrow">
            ${esc(dateStr)}
          </div>

          <h1 class="page-title">
            ${greeting}
          </h1>

          <p class="page-desc">
            Here's what is happening across your work.
          </p>

        </div>


        <button
          class="btn"
          id="dash-new-task"
        >
          + Add task
        </button>

      </div>



      <!-- =====================================================
           OVERDUE
           ===================================================== -->

      ${(overdue || []).length
        ? `

        <div
          class="card mb-3"
          style="border-color:rgba(248,113,113,.35)"
        >

          <div class="card-head">

            <div>

              <h3 style="color:var(--red)">
                Overdue
              </h3>

              <div class="sub">
                Tasks past their due date
              </div>

            </div>

          </div>


          <div>
            ${overdueRows}
          </div>

        </div>

      `
        : ""
      }



      <!-- =====================================================
           STATS
           ===================================================== -->

      <div class="stats">


        <div class="card card-pad stat">

          <div class="label">
            Open tasks
          </div>

          <div class="value">
            ${stats.openTasks}
          </div>

          <div class="note">
            ${stats.inProgress} in progress
          </div>

        </div>



        <div class="card card-pad stat">

          <div class="label">
            In Progress
          </div>

          <div class="value">
            ${stats.inProgress}
          </div>

          <div class="note note-blue">
            Active now
          </div>

        </div>



        <div class="card card-pad stat">

          <div class="label">
            Completed
          </div>

          <div class="value">
            ${stats.completedTasks}
          </div>

          <div class="note note-green">
            Total done
          </div>

        </div>



        <div class="card card-pad stat">

          <div class="label">
            Blocked
          </div>

          <div class="value">
            ${stats.blocked}
          </div>

          <div class="note note-red">
            Needs attention
          </div>

        </div>


      </div>



      <!-- =====================================================
           MAIN DASHBOARD GRID
           ===================================================== -->

      <div class="grid dashboard-grid">


        <!-- ===================================================
             ACTIVE WORK
             =================================================== -->

        <section class="card dashboard-card">

          <div class="card-head">

            <div>

              <h3>
                Active work
              </h3>

              <div class="sub">
                Tasks currently being worked on
              </div>

            </div>


            <button
              class="dashboard-link ml-auto"
              id="dash-view-tasks"
            >
              View all →
            </button>

          </div>


          <div>

            ${
              activeTasks.length
                ? activeTasks
                    .map((t) => {

                      const task = t.task || t;

                      const estimate =
                        task.estimatedMinutes
                          ? fmtDur(task.estimatedMinutes)
                          : "";

                      return `

                        <div
                          class="row-item dashboard-task-row"
                          onclick="window.openTask(${task.id})"
                        >

                          ${dot(task.status)}


                          <div class="grow">

                            <div class="row dashboard-task-top">

                              <span class="key">
                                ${esc(task.key)}
                              </span>

                              ${statusBadge(task.status)}

                            </div>


                            <div class="small dashboard-task-title">
                              ${esc(task.title)}
                            </div>


                            <div class="xs muted mt-1">
                              ${esc(t.projectName || t.projectKey || "")}
                            </div>

                          </div>


                          ${
                            estimate
                              ? `
                                <span class="xs muted dashboard-task-time">
                                  ${esc(estimate)}
                                </span>
                              `
                              : ""
                          }

                        </div>
                      `;
                    })
                    .join("")
                : `

                  <div class="empty dashboard-empty">

                    <div class="big">
                      No active tasks
                    </div>

                  </div>

                `
            }

          </div>

        </section>



        <!-- ===================================================
             PROJECT PROGRESS
             =================================================== -->

        <section class="card dashboard-card">

          <div class="card-head">

            <div>

              <h3>
                Project progress
              </h3>

              <div class="sub">
                Overall completion
              </div>

            </div>

          </div>


          <div class="card-pad stack dashboard-project-bars">

            ${
              projectBarsReal ||
              `

              <div class="empty dashboard-empty">

                <div class="big">
                  No projects yet
                </div>

              </div>

              `
            }

          </div>

        </section>



        <!-- ===================================================
             RECENT CHANGES
             =================================================== -->

        <section class="card dashboard-card">

          <div class="card-head">

            <div>

              <h3>
                Recent changes
              </h3>

              <div class="sub">
                A permanent history of work performed
              </div>

            </div>


            <button
              class="dashboard-link ml-auto"
              id="dash-open-history"
            >
              Open history →
            </button>

          </div>


          <div>

            ${
              recentActivity ||
              `

              <div class="empty dashboard-empty">

                <div class="big">
                  No recent history
                </div>

              </div>

              `
            }

          </div>

        </section>



        <!-- ===================================================
             QUICK WORK LOG
             =================================================== -->

        <section class="card dashboard-card">

          <div class="card-head">

            <div>

              <h3>
                Quick work log
              </h3>

              <div class="sub">
                Capture what you actually did
              </div>

            </div>

          </div>


          <div class="card-pad">

            <textarea
              id="dash-log-text"
              class="textarea dashboard-worklog-text"
              rows="5"
              placeholder="What did you work on?"
            ></textarea>


            <div class="row mt-2">

              <select
                id="dash-log-project"
                class="select grow"
              >

                <option value="">
                  General
                </option>

                ${projectOptions}

              </select>


              <button
                class="btn btn-primary"
                id="dash-log-save"
              >
                Log
              </button>

            </div>

          </div>

        </section>


      </div>

    </div>
  `;



  // ============================================================
  // QUICK WORK LOG
  // ============================================================

  document
    .getElementById("dash-log-save")
    ?.addEventListener("click", async () => {

      const text =
        document
          .getElementById("dash-log-text")
          ?.value
          .trim();

      const projectId =
        Number(
          document.getElementById("dash-log-project")?.value
        ) || null;


      if (!text) {
        return;
      }


      try {

        await call("create_work_log", {

          input: {
            description: text,
            duration_minutes: null,
            project_id: projectId,
          },

        });


        document.getElementById("dash-log-text").value = "";
        document.getElementById("dash-log-project").value = "";


        const { toast } =
          await import("./app.js");


        toast("Work logged");


        render(container);

      } catch (e) {

        const { toast } =
          await import("./app.js");


        toast(e.message, true);

      }

    });



  // ============================================================
  // ADD TASK
  // ============================================================

  document
    .getElementById("dash-new-task")
    ?.addEventListener("click", async () => {

      const { openNewTaskModal } =
        await import("./app.js");


      openNewTaskModal();

    });



  // ============================================================
  // VIEW ALL TASKS
  // ============================================================

  document
    .getElementById("dash-view-tasks")
    ?.addEventListener("click", async () => {

      const { switchPage } =
        await import("./app.js");


      switchPage("tasks");

    });



  // ============================================================
  // OPEN CHANGELOG
  // ============================================================

  document
    .getElementById("dash-open-history")
    ?.addEventListener("click", async () => {

      const { switchPage } =
        await import("./app.js");


      switchPage("changelog");

    });
}