// Scrapes https://kanbanflow.com/api-docs into per-page Markdown files.
// Usage: node scrape-kanbanflow-docs.mjs <output-dir>
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import TurndownService from "turndown";
import { gfm } from "turndown-plugin-gfm";

const outputDir = process.argv[2] ?? "kanbanflow-api-docs";
const baseUrl = "https://kanbanflow.com";

const turndown = new TurndownService({
  headingStyle: "atx",
  codeBlockStyle: "fenced",
});
turndown.use(gfm);

async function fetchHtml(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${response.status} ${url}`);
  return response.text();
}

function extractMainContent(html) {
  // The docs content lives after the sidebar nav; grab the article/main region.
  // Pages are server-rendered; the content column is the last major div before the footer.
  const bodyMatch = html.match(/<body[^>]*>([\s\S]*?)<\/body>/);
  let body = bodyMatch ? bodyMatch[1] : html;
  // Drop header, nav and footer blocks.
  body = body
    .replace(/<header[\s\S]*?<\/header>/g, "")
    .replace(/<nav[\s\S]*?<\/nav>/g, "")
    .replace(/<footer[\s\S]*?<\/footer>/g, "")
    .replace(/<script[\s\S]*?<\/script>/g, "")
    .replace(/<style[\s\S]*?<\/style>/g, "");
  return body;
}

const indexHtml = await fetchHtml(`${baseUrl}/api-docs`);
const slugs = [
  ...new Set(
    [...indexHtml.matchAll(/href="(\/api-docs(?:\/[a-z0-9-]+)?)"/g)].map(
      (match) => match[1],
    ),
  ),
];

await mkdir(outputDir, { recursive: true });
const indexLines = ["# KanbanFlow API documentation (scraped)", ""];

for (const slug of slugs) {
  const url = `${baseUrl}${slug}`;
  const html = await fetchHtml(url);
  const titleMatch = html.match(/<title>([^<]*)<\/title>/);
  const title = (titleMatch ? titleMatch[1] : slug)
    .replace(/ - API documentation - KanbanFlow/, "")
    .trim();
  const markdown = turndown.turndown(extractMainContent(html));
  const fileName =
    slug === "/api-docs" ? "index.md" : `${slug.replace("/api-docs/", "")}.md`;
  const header = `# ${title}\n\n> Source: ${url}\n> Scraped: ${new Date().toISOString().slice(0, 10)}\n\n`;
  await writeFile(path.join(outputDir, fileName), header + markdown + "\n");
  indexLines.push(`- [${title}](${fileName})`);
  console.log(`saved ${fileName}`);
}

await writeFile(path.join(outputDir, "README.md"), indexLines.join("\n") + "\n");
console.log(`\n${slugs.length} pages -> ${outputDir}`);
