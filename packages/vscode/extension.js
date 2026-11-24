/**
 * himari-formula — hover 公式渲染（sidecar 车道，零 wasm）。
 *
 * 协议：`himari-fmt --sidecar foo.formula` 生成 `foo.formula.json`
 * （每条公式的 latex + mathml，checked-in，漂移门覆盖）。本 provider 在
 * hover 时读 sidecar，把该行所属公式的 latex 塞进 `$$…$$` —— VS Code
 * hover 的 markdown 内置 KaTeX 渲染，于是公式以排版形态出现。
 */
const vscode = require('vscode');
const fs = require('fs');
const path = require('path');
const { previewCommand } = require('./preview');
const diagnosticsFeature = require('./diagnostics');

/** @type {Map<string, {items: Array<{name: string, latex: string, mathml: string}>}>} */
const sidecars = new Map();

function loadSidecar(doc) {
  if (doc.languageId !== 'himari-formula') return;
  const side = doc.uri.fsPath + '.json';
  try {
    sidecars.set(doc.uri.fsPath, JSON.parse(fs.readFileSync(side, 'utf8')));
  } catch {
    sidecars.delete(doc.uri.fsPath); // 无 sidecar = hover 静默不出现
  }
}

/** 找光标行所属的公式：从该行向上/向下找最近的赋值行（`name =`）或矩阵首行（`[`）。 */
function itemAt(items, line, doc) {
  if (!items || !items.items || items.items.length === 0) return undefined;
  // 无名公式（模式矩阵等）→ 取第一条（.formula 一文件一公式的现行约定）
  const named = items.items.filter((x) => x.name);
  if (named.length === 0) return items.items[0];
  // 向上找最近的 `name =` 行
  for (let l = line; l >= 0; l--) {
    const text = doc.lineAt(l).text;
    const m = text.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=/);
    if (m) {
      const hit = items.items.find((x) => x.name === m[1]);
      if (hit) return hit;
    }
  }
  return undefined;
}

function activate(context) {
  const sel = { language: 'himari-formula' };
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(loadSidecar),
    vscode.workspace.onDidChangeTextDocument((e) => loadSidecar(e.document)),
    vscode.languages.registerHoverProvider(sel, {
      provideHover(doc, pos) {
        loadSidecar(doc);
        const items = sidecars.get(doc.uri.fsPath);
        const item = itemAt(items, pos.line, doc);
        if (!item) return undefined;
        const name = item.name ? `**${item.name}** = ` : '';
        const md = new vscode.MarkdownString(`${name}$${item.latex}$`);
        // sidecar 现在带量纲与参数：hover 里直接给出签名信息，不用切去公式源
        const meta = [];
        if (item.dim && item.dim !== '?') meta.push(`量纲: \`${item.dim}\``);
        if (Array.isArray(item.params) && item.params.length > 0) {
          meta.push(`参数: ${item.params.map((p) => `\`${p}\``).join(', ')}`);
        }
        if (meta.length > 0) md.appendMarkdown(`\n\n${meta.join(' ｜ ')}`);
        return new vscode.Hover(md);
      },
    })
  );
  // 诊断 + 补全（CLI 车道：himari-fmt --diagnose / --symbols）
  diagnosticsFeature.activate(context);

  // L2 侧栏预览命令
  context.subscriptions.push(
    vscode.commands.registerCommand('himari-formula.preview', previewCommand(context))
  );

  // 打开时已开着的文档
  if (vscode.window.activeTextEditor) loadSidecar(vscode.window.activeTextEditor.document);
}

module.exports = { activate };