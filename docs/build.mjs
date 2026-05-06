import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const docsDir = path.dirname(__filename);

const pages = [
  { source: "README.md", output: "index.html", title: "Overview", eyebrow: "Documentation" },
  { source: "product.md", output: "product.html", title: "Product Direction", eyebrow: "Vision" },
  { source: "architecture.md", output: "architecture.html", title: "Architecture", eyebrow: "System" },
  { source: "workflow.md", output: "workflow.html", title: "Workflow", eyebrow: "Concepts" },
  { source: "agents.md", output: "agents.html", title: "Agents", eyebrow: "Concepts" },
  { source: "sandbox.md", output: "sandbox.html", title: "Sandbox", eyebrow: "Runtime" },
  { source: "cli-and-skill.md", output: "cli-and-skill.html", title: "CLI and Skill", eyebrow: "Interfaces" },
  { source: "roadmap.md", output: "roadmap.html", title: "Roadmap", eyebrow: "Planning" },
  { source: "github-pages.md", output: "github-pages.html", title: "GitHub Pages", eyebrow: "Publishing" },
];

const SITE_TITLE = "Manual Docs";
const SITE_TAGLINE = "Rust-based, fast, lightweight workflow automation for agents.";
const REPO_URL = "https://github.com/BEOKS/Manual";

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

function inlineMarkdown(value) {
  let output = escapeHtml(value);
  output = output.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  output = output.replace(/(?<!\*)\*([^*]+)\*(?!\*)/g, "<em>$1</em>");
  output = output.replace(/`([^`]+)`/g, "<code>$1</code>");
  output = output.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label, href) => {
    const safeHref = escapeAttr(href.replace(/\.md(#.*)?$/, ".html$1"));
    const isExternal = /^https?:\/\//.test(href);
    const rel = isExternal ? ' rel="noopener noreferrer" target="_blank"' : "";
    return `<a href="${safeHref}"${rel}>${label}</a>`;
  });
  return output;
}

function slugify(value) {
  return value
    .toLowerCase()
    .replace(/<[^>]+>/g, "")
    .replace(/[`*_]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function isTableStart(lines, index) {
  return (
    lines[index]?.trim().startsWith("|") &&
    lines[index + 1]?.trim().startsWith("|") &&
    /^[\s|:-]+$/.test(lines[index + 1].trim())
  );
}

function renderTable(lines, start) {
  const rows = [];
  let index = start;

  while (index < lines.length && lines[index].trim().startsWith("|")) {
    rows.push(
      lines[index]
        .trim()
        .replace(/^\||\|$/g, "")
        .split("|")
        .map((cell) => cell.trim()),
    );
    index += 1;
  }

  const header = rows[0];
  const body = rows.slice(2);
  const thead = `<thead><tr>${header.map((cell) => `<th>${inlineMarkdown(cell)}</th>`).join("")}</tr></thead>`;
  const tbody = `<tbody>${body
    .map((row) => `<tr>${row.map((cell) => `<td>${inlineMarkdown(cell)}</td>`).join("")}</tr>`)
    .join("")}</tbody>`;

  return { html: `<div class="table-wrap"><table>${thead}${tbody}</table></div>`, next: index };
}

const CALLOUT_KINDS = {
  note: { icon: "i", title: "Note" },
  tip: { icon: "✓", title: "Tip" },
  warning: { icon: "!", title: "Warning" },
  danger: { icon: "!", title: "Danger" },
  important: { icon: "!", title: "Important" },
};

function detectCallout(blockLines) {
  if (blockLines.length === 0) return null;
  const first = blockLines[0];
  const m = /^\s*\[!(NOTE|TIP|WARNING|DANGER|IMPORTANT)\]\s*(.*)$/i.exec(first);
  if (!m) return null;
  const key = m[1].toLowerCase();
  const kind = key === "important" ? "warning" : key === "danger" ? "danger" : key;
  const customTitle = m[2]?.trim();
  const rest = blockLines.slice(1);
  return {
    kind,
    title: customTitle || CALLOUT_KINDS[kind]?.title || CALLOUT_KINDS[key]?.title || "Note",
    icon: CALLOUT_KINDS[kind]?.icon || "i",
    body: rest,
  };
}

function renderBlockquote(lines, start) {
  const blockLines = [];
  let index = start;
  while (index < lines.length && lines[index].trim().startsWith(">")) {
    blockLines.push(lines[index].replace(/^\s*>\s?/, ""));
    index += 1;
  }
  const callout = detectCallout(blockLines);
  if (callout) {
    const inner = renderMarkdownBlocks(callout.body);
    return {
      html: `<aside class="callout ${callout.kind}"><div class="callout-icon">${escapeHtml(callout.icon)}</div><div class="callout-body"><div class="callout-title">${escapeHtml(callout.title)}</div>${inner}</div></aside>`,
      next: index,
    };
  }
  const inner = renderMarkdownBlocks(blockLines);
  return { html: `<blockquote>${inner}</blockquote>`, next: index };
}

function renderCodeBlock(language, code) {
  const langLabel = language ? language.toUpperCase() : "TEXT";
  const langClass = language ? `language-${escapeAttr(language)}` : "language-text";
  const hasMeta = true;
  const metaHtml = hasMeta
    ? `<div class="code-block-meta"><span class="lang">${escapeHtml(langLabel)}</span><button type="button" class="copy-btn" aria-label="Copy code">Copy</button></div>`
    : "";
  return `<div class="code-block${hasMeta ? "" : " no-meta"}">${metaHtml}<pre><code class="${langClass}">${escapeHtml(code)}</code></pre></div>`;
}

function collectHeadings(html) {
  const headings = [];
  const re = /<h([23])\s+id="([^"]+)"[^>]*>([\s\S]*?)<\/h\1>/g;
  let m;
  while ((m = re.exec(html)) !== null) {
    const level = Number(m[1]);
    const id = m[2];
    const inner = m[3].replace(/<a class="heading-anchor"[\s\S]*?<\/a>/g, "");
    const text = inner.replace(/<[^>]+>/g, "").trim();
    headings.push({ level, id, text });
  }
  return headings;
}

function renderToc(headings) {
  if (headings.length === 0) {
    return '<div class="toc-empty"></div>';
  }
  const items = headings
    .map((h) => `<li class="toc-h${h.level}"><a href="#${escapeAttr(h.id)}" data-toc-id="${escapeAttr(h.id)}">${escapeHtml(h.text)}</a></li>`)
    .join("");
  return `<div class="toc-title">On this page</div><ul class="toc-list">${items}</ul>`;
}

function extractDescription(markdown) {
  const lines = markdown.split(/\r?\n/);
  let inCode = false;
  for (const line of lines) {
    const t = line.trim();
    if (t.startsWith("```")) {
      inCode = !inCode;
      continue;
    }
    if (inCode) continue;
    if (!t) continue;
    if (t.startsWith("#")) continue;
    if (t.startsWith("|") || t.startsWith(">") || t.startsWith("- ") || /^\d+\.\s/.test(t)) continue;
    const plain = t
      .replace(/`([^`]+)`/g, "$1")
      .replace(/\*\*([^*]+)\*\*/g, "$1")
      .replace(/\*([^*]+)\*/g, "$1")
      .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
    return plain.length > 180 ? plain.slice(0, 177) + "…" : plain;
  }
  return SITE_TAGLINE;
}

function extractFirstParagraphAsLead(markdownLines) {
  // Returns { lead: string|null, remainingLines: string[] }
  // Lead = first paragraph immediately after the H1 (if any), or first paragraph.
  const result = [];
  let i = 0;
  let foundH1 = false;
  while (i < markdownLines.length) {
    const line = markdownLines[i];
    const trimmed = line.trim();
    if (!foundH1 && /^#\s+/.test(trimmed)) {
      foundH1 = true;
      result.push(line);
      i += 1;
      // skip blank lines
      while (i < markdownLines.length && markdownLines[i].trim() === "") {
        result.push(markdownLines[i]);
        i += 1;
      }
      // capture paragraph
      const para = [];
      while (
        i < markdownLines.length &&
        markdownLines[i].trim() !== "" &&
        !/^[#>\-|]/.test(markdownLines[i].trim()) &&
        !/^\d+\.\s/.test(markdownLines[i].trim()) &&
        !markdownLines[i].trim().startsWith("```")
      ) {
        para.push(markdownLines[i]);
        i += 1;
      }
      if (para.length > 0) {
        return {
          leadLines: para,
          remainingLines: [...result, ...markdownLines.slice(i)],
        };
      }
      return { leadLines: null, remainingLines: markdownLines };
    }
    result.push(line);
    i += 1;
  }
  return { leadLines: null, remainingLines: markdownLines };
}

function renderMarkdownBlocks(lines) {
  const html = [];
  let index = 0;
  let inList = false;
  let inOrderedList = false;
  let paragraph = [];

  function flushParagraph() {
    if (paragraph.length > 0) {
      html.push(`<p>${inlineMarkdown(paragraph.join(" "))}</p>`);
      paragraph = [];
    }
  }

  function closeList() {
    if (inList) {
      html.push("</ul>");
      inList = false;
    }
    if (inOrderedList) {
      html.push("</ol>");
      inOrderedList = false;
    }
  }

  while (index < lines.length) {
    const line = lines[index];
    const trimmed = line.trim();

    if (trimmed.startsWith("```")) {
      flushParagraph();
      closeList();
      const language = trimmed.slice(3).trim();
      const code = [];
      index += 1;
      while (index < lines.length && !lines[index].trim().startsWith("```")) {
        code.push(lines[index]);
        index += 1;
      }
      html.push(renderCodeBlock(language, code.join("\n")));
      index += 1;
      continue;
    }

    if (isTableStart(lines, index)) {
      flushParagraph();
      closeList();
      const table = renderTable(lines, index);
      html.push(table.html);
      index = table.next;
      continue;
    }

    if (trimmed.startsWith(">")) {
      flushParagraph();
      closeList();
      const bq = renderBlockquote(lines, index);
      html.push(bq.html);
      index = bq.next;
      continue;
    }

    if (trimmed === "---") {
      flushParagraph();
      closeList();
      html.push("<hr>");
      index += 1;
      continue;
    }

    if (trimmed === "") {
      flushParagraph();
      closeList();
      index += 1;
      continue;
    }

    const heading = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1].length;
      const text = heading[2];
      const slug = slugify(text);
      const inner = inlineMarkdown(text);
      const anchor = level <= 3
        ? ` <a class="heading-anchor" href="#${escapeAttr(slug)}" aria-label="Link to this section">#</a>`
        : "";
      html.push(`<h${level} id="${escapeAttr(slug)}">${inner}${anchor}</h${level}>`);
      index += 1;
      continue;
    }

    const unordered = /^-\s+(.+)$/.exec(trimmed);
    if (unordered) {
      flushParagraph();
      if (inOrderedList) {
        html.push("</ol>");
        inOrderedList = false;
      }
      if (!inList) {
        html.push("<ul>");
        inList = true;
      }
      html.push(`<li>${inlineMarkdown(unordered[1])}</li>`);
      index += 1;
      continue;
    }

    const ordered = /^\d+\.\s+(.+)$/.exec(trimmed);
    if (ordered) {
      flushParagraph();
      if (inList) {
        html.push("</ul>");
        inList = false;
      }
      if (!inOrderedList) {
        html.push("<ol>");
        inOrderedList = true;
      }
      html.push(`<li>${inlineMarkdown(ordered[1])}</li>`);
      index += 1;
      continue;
    }

    paragraph.push(trimmed);
    index += 1;
  }

  flushParagraph();
  closeList();
  return html.join("\n");
}

function renderMarkdown(markdown) {
  const allLines = markdown.split(/\r?\n/);
  // Try to extract first paragraph after H1 as a lead. If found, render it specially.
  const { leadLines, remainingLines } = extractFirstParagraphAsLead(allLines);
  if (leadLines) {
    // Find the H1, render H1 + lead inline, then the rest.
    const beforeH1 = [];
    let i = 0;
    while (i < remainingLines.length && !/^#\s+/.test(remainingLines[i].trim())) {
      beforeH1.push(remainingLines[i]);
      i += 1;
    }
    const h1Line = remainingLines[i] || "";
    const afterLeadStart = i + 1;
    // Skip blank lines and the captured lead lines from remaining
    let j = afterLeadStart;
    while (j < remainingLines.length && remainingLines[j].trim() === "") j += 1;
    // The lead lines are followed by the remaining; they were originally between blanks.
    // Since extractFirstParagraphAsLead returned `remainingLines` already without lead,
    // just render: beforeH1 + h1 + rest after position j.
    const rest = remainingLines.slice(j);
    const headBlock = renderMarkdownBlocks([...beforeH1, h1Line]);
    const leadHtml = `<p class="lead">${inlineMarkdown(leadLines.map((l) => l.trim()).join(" "))}</p>`;
    const restHtml = renderMarkdownBlocks(rest);
    return headBlock + "\n" + leadHtml + "\n" + restHtml;
  }
  return renderMarkdownBlocks(allLines);
}

function nav(activeOutput) {
  return pages
    .map((page) => {
      const active = page.output === activeOutput ? ' class="active"' : "";
      return `        <a${active} href="${page.output}">${escapeHtml(page.title)}</a>`;
    })
    .join("\n");
}

function paginationFor(currentIndex) {
  const prev = currentIndex > 0 ? pages[currentIndex - 1] : null;
  const next = currentIndex < pages.length - 1 ? pages[currentIndex + 1] : null;
  if (!prev && !next) return "";
  const prevHtml = prev
    ? `<a class="prev" href="${prev.output}"><span class="label">← Previous</span><span class="title">${escapeHtml(prev.title)}</span></a>`
    : "";
  const nextHtml = next
    ? `<a class="next" href="${next.output}"><span class="label">Next →</span><span class="title">${escapeHtml(next.title)}</span></a>`
    : "";
  return `<nav class="pagination" aria-label="Page navigation">${prevHtml}${nextHtml}</nav>`;
}

function inlineScript() {
  // Inline JS: theme toggle, copy buttons, TOC active highlighting.
  return `
(function () {
  // ----- Theme -----
  var STORAGE_KEY = "manual-docs-theme";
  var root = document.documentElement;
  try {
    var saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "dark" || saved === "light") root.setAttribute("data-theme", saved);
  } catch (e) {}
  function setTheme(t) {
    root.setAttribute("data-theme", t);
    try { localStorage.setItem(STORAGE_KEY, t); } catch (e) {}
  }
  document.addEventListener("click", function (e) {
    var btn = e.target.closest("[data-theme-toggle]");
    if (!btn) return;
    var current = root.getAttribute("data-theme");
    if (!current) {
      var prefersDark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
      current = prefersDark ? "dark" : "light";
    }
    setTheme(current === "dark" ? "light" : "dark");
  });

  // ----- Mobile menu -----
  document.addEventListener("click", function (e) {
    var btn = e.target.closest("[data-menu-toggle]");
    if (!btn) return;
    var sidebar = document.querySelector(".sidebar");
    if (sidebar) sidebar.classList.toggle("open");
  });

  // ----- Copy buttons -----
  document.addEventListener("click", function (e) {
    var btn = e.target.closest(".copy-btn");
    if (!btn) return;
    var block = btn.closest(".code-block");
    if (!block) return;
    var code = block.querySelector("pre code");
    if (!code) return;
    var text = code.innerText;
    var done = function () {
      var prev = btn.textContent;
      btn.textContent = "Copied";
      btn.classList.add("copied");
      setTimeout(function () {
        btn.textContent = prev;
        btn.classList.remove("copied");
      }, 1400);
    };
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done).catch(function () {
        // Fallback
        var ta = document.createElement("textarea");
        ta.value = text;
        document.body.appendChild(ta);
        ta.select();
        try { document.execCommand("copy"); done(); } catch (err) {}
        document.body.removeChild(ta);
      });
    } else {
      var ta2 = document.createElement("textarea");
      ta2.value = text;
      document.body.appendChild(ta2);
      ta2.select();
      try { document.execCommand("copy"); done(); } catch (err) {}
      document.body.removeChild(ta2);
    }
  });

  // ----- TOC active highlighting -----
  var tocLinks = Array.prototype.slice.call(document.querySelectorAll(".toc-list a[data-toc-id]"));
  if (tocLinks.length > 0 && "IntersectionObserver" in window) {
    var byId = {};
    tocLinks.forEach(function (a) { byId[a.getAttribute("data-toc-id")] = a; });
    var visible = new Set();
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) visible.add(entry.target.id);
        else visible.delete(entry.target.id);
      });
      var firstVisible = null;
      Object.keys(byId).some(function (id) {
        if (visible.has(id)) { firstVisible = id; return true; }
        return false;
      });
      if (!firstVisible && tocLinks.length > 0) {
        // Pick the last heading scrolled past
        var passed = null;
        Object.keys(byId).forEach(function (id) {
          var el = document.getElementById(id);
          if (el && el.getBoundingClientRect().top < 120) passed = id;
        });
        firstVisible = passed;
      }
      tocLinks.forEach(function (a) { a.classList.remove("active"); });
      if (firstVisible && byId[firstVisible]) byId[firstVisible].classList.add("active");
    }, { rootMargin: "-80px 0px -65% 0px", threshold: 0 });
    Object.keys(byId).forEach(function (id) {
      var el = document.getElementById(id);
      if (el) observer.observe(el);
    });
  }
})();
`.trim();
}

function renderPage(page, body, headings, currentIndex, description) {
  const tocHtml = renderToc(headings);
  const pagination = paginationFor(currentIndex);
  const eyebrow = page.eyebrow
    ? `<div class="eyebrow">${escapeHtml(page.eyebrow)}</div>`
    : "";

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>${escapeHtml(SITE_TITLE)} — ${escapeHtml(page.title)}</title>
  <meta name="description" content="${escapeAttr(description)}">
  <meta property="og:title" content="${escapeAttr(SITE_TITLE + " — " + page.title)}">
  <meta property="og:description" content="${escapeAttr(description)}">
  <meta property="og:type" content="article">
  <meta name="twitter:card" content="summary">
  <link rel="stylesheet" href="assets/site.css">
  <script>
    (function () {
      try {
        var t = localStorage.getItem("manual-docs-theme");
        if (t === "dark" || t === "light") document.documentElement.setAttribute("data-theme", t);
      } catch (e) {}
    })();
  </script>
</head>
<body>
  <div class="shell">
    <aside class="sidebar">
      <a class="brand" href="index.html">
        <div class="brand-mark">M</div>
        <div class="brand-name">Manual</div>
      </a>
      <p class="tagline">${escapeHtml(SITE_TAGLINE)}</p>
      <button type="button" class="menu-toggle" data-menu-toggle aria-label="Toggle navigation">☰ Menu</button>
      <div class="sidebar-nav-wrap">
        <div class="nav-title">Documentation</div>
        <nav class="nav" aria-label="Documentation navigation">
${nav(page.output)}
        </nav>
        <div class="sidebar-footer">
          <button type="button" class="theme-toggle" data-theme-toggle aria-label="Toggle color theme">
            <svg class="theme-icon-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"></path></svg>
            <svg class="theme-icon-dark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>
            <span>Theme</span>
          </button>
          <a class="repo-link" href="${escapeAttr(REPO_URL)}" rel="noopener noreferrer" target="_blank" aria-label="GitHub repository">
            <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2c-3.34.73-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.74.08-.73.08-.73 1.21.09 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.5.99.11-.78.42-1.3.76-1.6-2.66-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.13-.31-.54-1.55.11-3.23 0 0 1-.32 3.3 1.23a11.46 11.46 0 0 1 6 0c2.29-1.55 3.3-1.23 3.3-1.23.65 1.68.24 2.92.11 3.23.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.62-5.49 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .5z"/></svg>
            GitHub
          </a>
        </div>
      </div>
    </aside>

    <main class="content">
      <article>
        ${eyebrow}
        ${body}
        ${pagination}
        <footer class="footer">
          <span>Generated from Markdown by <code>docs/build.mjs</code>.</span>
          <span>${escapeHtml(SITE_TITLE)}</span>
        </footer>
      </article>
    </main>

    <aside class="toc" aria-label="On this page">
      ${tocHtml}
    </aside>
  </div>
  <script>${inlineScript()}</script>
</body>
</html>
`;
}

for (let i = 0; i < pages.length; i += 1) {
  const page = pages[i];
  const sourcePath = path.join(docsDir, page.source);
  const outputPath = path.join(docsDir, page.output);
  const markdown = fs.readFileSync(sourcePath, "utf8");
  const description = extractDescription(markdown);
  const body = renderMarkdown(markdown);
  const headings = collectHeadings(body);
  fs.writeFileSync(outputPath, renderPage(page, body, headings, i, description));
  console.log(`wrote ${path.relative(process.cwd(), outputPath)}`);
}
