// Minimal dependency-free Markdown -> HTML renderer.
// Output is escaped first, then transformed. No raw HTML is passed through.

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function safeUrl(url) {
  const u = url.trim();
  if (/^(https?:|mailto:|#|\/)/i.test(u)) return u;
  return "#";
}

function inline(escaped) {
  let s = escaped
    // inline code
    .replace(/`([^`]+)`/g, (_, c) => `<code>${c}</code>`)
    // bold
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    // italic
    .replace(/\*([^*]+)\*/g, "<em>$1</em>")
    // strike
    .replace(/~~([^~]+)~~/g, "<del>$1</del>")
    // links [text](url)
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, t, u) => `<a href="${safeUrl(u)}" target="_blank" rel="noopener">${t}</a>`);
  return s;
}

function buildList(entries, ordered) {
  const tag = ordered ? "ol" : "ul";
  return `<${tag}>${entries.map((e) => `<li>${inline(e)}</li>`).join("")}</${tag}>`;
}

function buildTable(rows) {
  if (rows.length < 2) return "";
  const head = rows[0].split("|").map((c) => c.trim()).filter((c) => c !== "");
  const body = rows
    .slice(1)
    .map((r) => {
      const cells = r.split("|").map((c) => c.trim()).filter((c) => c !== "");
      return `<tr>${cells.map((c) => `<td>${inline(c)}</td>`).join("")}</tr>`;
    })
    .join("");
  return `<table><thead><tr>${head.map((h) => `<th>${inline(h)}</th>`).join("")}</tr></thead><tbody>${body}</tbody></table>`;
}

export function renderMarkdown(src) {
  const text = String(src || "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  if (!text.trim()) return "";

  const lines = text.split("\n");
  const out = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // fenced code block
    if (/^```/.test(line)) {
      const lang = line.slice(3).trim();
      const buf = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) {
        buf.push(lines[i]);
        i++;
      }
      i++; // skip closing fence
      const cls = lang ? ` class="lang-${escapeHtml(lang)}"` : "";
      out.push(`<pre><code${cls}>${escapeHtml(buf.join("\n"))}</code></pre>`);
      continue;
    }

    // table (consecutive | lines with a separator row)
    if (line.includes("|") && i + 1 < lines.length && /^\s*\|?[\s:|-]+\|?\s*$/.test(lines[i + 1]) && lines[i + 1].includes("-")) {
      const rows = [line];
      i++;
      while (i < lines.length && lines[i].includes("|")) {
        rows.push(lines[i]);
        i++;
      }
      out.push(buildTable(rows));
      continue;
    }

    // heading
    const h = line.match(/^(#{1,6})\s+(.*)$/);
    if (h) {
      const level = h[1].length;
      out.push(`<h${level}>${inline(escapeHtml(h[2]))}</h${level}>`);
      i++;
      continue;
    }

    // horizontal rule
    if (/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      out.push("<hr>");
      i++;
      continue;
    }

    // blockquote: consecutive > lines
    if (/^\s*>\s?/.test(line)) {
      const buf = [];
      while (i < lines.length && /^\s*>\s?/.test(lines[i])) {
        buf.push(lines[i].replace(/^\s*>\s?/, ""));
        i++;
      }
      out.push(`<blockquote>${renderMarkdown(buf.join("\n"))}</blockquote>`);
      continue;
    }

    // list: consecutive - * + or numbered lines
    const listMatch = line.match(/^(\s*)([-*+]|\d+\.)\s+(.*)$/);
    if (listMatch) {
      const ordered = /\d+\./.test(listMatch[2]);
      const entries = [listMatch[3]];
      const indent = listMatch[1];
      i++;
      while (i < lines.length) {
        const m = lines[i].match(/^(\s*)([-*+]|\d+\.)\s+(.*)$/);
        if (!m || m[1] !== indent) break;
        entries.push(m[3]);
        i++;
      }
      out.push(buildList(entries, ordered));
      continue;
    }

    // paragraph: gather until blank line or new block
    const buf = [];
    while (i < lines.length && lines[i].trim() !== "" && !/^(#{1,6}\s|```|>\s?|---)/.test(lines[i]) && !/^\s*([-*+]|\d+\.)\s+/.test(lines[i])) {
      buf.push(lines[i].trim());
      i++;
    }
    if (buf.length) {
      out.push(`<p>${inline(escapeHtml(buf.join(" ")))}</p>`);
      continue;
    }

    // blank line
    i++;
  }

  return out.join("\n");
}