/**
 * himari-formula — L2 侧栏预览：当前 .formula 的全部公式按 MathML 排版。
 *
 * webview 原生支持 MathML Core（Chromium 内核），sidecar 里的 mathml 字段
 * 直接可渲染 —— 与 hover（KaTeX/$latex$）同一份 sidecar 真源。
 */
const vscode = require('vscode');
const fs = require('fs');

function previewCommand(context) {
  return () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'himari-formula') {
      vscode.window.showWarningMessage('himari：当前编辑器不是 .formula 文件');
      return;
    }
    const side = editor.document.uri.fsPath + '.json';
    let data;
    try {
      data = JSON.parse(fs.readFileSync(side, 'utf8'));
    } catch {
      vscode.window.showWarningMessage(
        'himari：读不到 sidecar（先跑 himari-fmt --sidecar 生成）'
      );
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      'himari-preview',
      `himari — ${require('path').basename(editor.document.fileName)}`,
      vscode.ViewColumn.Beside,
      {}
    );
    // 每条公式带源行号 → 可点击跳回源码（双向定位）
    // 每条公式带源行号 → 可点击跳回源码；量纲与参数与 hover 同源（sidecar）
    const rows = data.items
      .map((it) => {
        const meta = [];
        if (it.dim && it.dim !== '?') meta.push(`[${it.dim}]`);
        if (Array.isArray(it.params) && it.params.length > 0) {
          meta.push(`参数 ${it.params.join(', ')}`);
        }
        const tail = meta.length ? `<span class="meta">${meta.join(' ')}</span>` : '';
        return `<div class="f" data-line="${it.line || 0}" title="跳到第 ${it.line} 行">${
          it.name ? `<code>${it.name}</code> =` : ''
        }${it.mathml}${tail}</div>`;
      })
      .join('\n');
    panel.webview.onDidReceiveMessage((msg) => {
      if (!msg || !msg.line) return;
      const ed = vscode.window.activeTextEditor;
      if (!ed || ed.document.uri.fsPath !== editor.document.uri.fsPath) return;
      const pos = new vscode.Position(Math.max(0, msg.line - 1), 0);
      ed.selection = new vscode.Selection(pos, pos);
      ed.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
    });
    panel.webview.html = `<!DOCTYPE html>
<html><head><style>
  body { font-family: var(--vscode-font-family); padding: 12px; }
  .f { margin: 14px 0; font-size: 120%; }
  code { color: var(--vscode-textPreformat-foreground); margin-right: 8px; }
  .meta { color: var(--vscode-descriptionForeground); font-size: 80%; margin-left: 10px; }
</style></head>
<body>${rows}
<script>
  const vscodeApi = acquireVsCodeApi();
  document.querySelectorAll('.f').forEach((el) => {
    const line = parseInt(el.dataset.line || '0', 10);
    if (!line) return;
    el.style.cursor = 'pointer';
    el.addEventListener('click', () => vscodeApi.postMessage({ line }));
  });
</script>
</body></html>`;
  };
}

module.exports = { previewCommand };