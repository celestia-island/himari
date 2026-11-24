/**
 * himari-formula — 诊断与补全（CLI 车道）。
 *
 * 这两条能力不需要语言服务器：`himari-fmt` 已经是同一套解析器的可执行入口。
 *   - 诊断：`himari-fmt --diagnose <file>` 解析 + 量纲 + 形状，非零退出即有问题，
 *     输出形如 `<path>:<line>: <名字> 量纲错误：…`；
 *   - 补全：`himari-fmt --symbols <file>` 每行 `<kind> <name>`。
 *
 * 为什么不做 LSP：LSP 要手写 JSON-RPC 与文档同步，而当前所有诊断都是**整文件**
 * 级别的，保存/打开时跑一次 CLI 就够。等到需要「边打字边报错」再上 LSP，
 * 那时这份 CLI 契约可以直接复用。
 */
const vscode = require('vscode');
const { execFile } = require('child_process');

/** 找可用的 himari-fmt：配置项 → PATH。 */
function fmtBin() {
  return vscode.workspace
    .getConfiguration('himari')
    .get('fmtPath', 'himari-fmt');
}

function runFmt(args, cwd) {
  return new Promise((resolve) => {
    execFile(fmtBin(), args, { cwd, timeout: 20000 }, (err, stdout, stderr) => {
      resolve({ ok: !err, stdout: stdout || '', stderr: stderr || '' });
    });
  });
}

const diagnostics = vscode.languages.createDiagnosticCollection('himari');

/** `<path>:<line>: <msg>` / `<path>:<line>:<col>: <msg>` → Diagnostic。 */
function parseDiagnostics(text, doc) {
  const out = [];
  for (const line of text.split(/\r?\n/)) {
    if (!line.trim()) continue;
    const m = line.match(/^(.*?):(\d+)(?::(\d+))?:\s*(.*)$/);
    if (!m) continue;
    const lineNo = Math.max(0, Number(m[2]) - 1);
    const col = m[3] ? Math.max(0, Number(m[3]) - 1) : 0;
    const safeLine = Math.min(lineNo, Math.max(0, doc.lineCount - 1));
    const textLine = doc.lineAt(safeLine).text;
    const range = new vscode.Range(safeLine, col, safeLine, Math.max(col + 1, textLine.length));
    out.push(new vscode.Diagnostic(range, m[4], vscode.DiagnosticSeverity.Error));
  }
  return out;
}

async function diagnose(doc) {
  if (doc.languageId !== 'himari-formula') return;
  const fsPath = doc.uri.fsPath;
  const { stderr, stdout } = await runFmt(['--diagnose', fsPath], require('path').dirname(fsPath));
  const text = `${stderr}\n${stdout}`.trim();
  diagnostics.set(doc.uri, parseDiagnostics(text, doc));
}

/** 文件里的 decl / item 名（补全候选）。 */
async function symbols(doc) {
  const fsPath = doc.uri.fsPath;
  const { stdout } = await runFmt(['--symbols', fsPath], require('path').dirname(fsPath));
  const out = [];
  for (const line of stdout.split(/\r?\n/)) {
    const [kind, name] = line.trim().split(/\s+/);
    if (name) out.push({ kind, name });
  }
  return out;
}

/** 公式里可用的函数与算符（补全的第二类候选）。 */
const FUNCTIONS = [
  'exp', 'ln', 'log', 'sqrt', 'sin', 'cos', 'tan', 'sinh', 'cosh', 'tanh',
  'abs', 'arg', 'conj', 're', 'im',
];

/**
 * 可选的 LSP 通道：配置了 `himari.lspPath` 就用 `himari-lsp`（边打字边诊断），
 * 否则退回 CLI 保存时诊断。两条路共用同一套解析器，结论一致。
 */
function startLsp(context) {
  const bin = vscode.workspace.getConfiguration('himari').get('lspPath', '');
  if (!bin) return;
  const { LanguageClient, TransportKind } = (() => {
    try {
      // 只有装了 vscode-languageclient 才启用；没装就静默退回 CLI
      return require('vscode-languageclient/node');
    } catch {
      return {};
    }
  })();
  if (!LanguageClient) {
    console.log('himari: 未安装 vscode-languageclient，退回 CLI 诊断');
    return;
  }
  const client = new LanguageClient(
    'himari',
    'Himari Formula',
    { command: bin, transport: TransportKind.stdio },
    { documentSelector: [{ language: 'himari-formula' }] }
  );
  client.start();
  context.subscriptions.push({ dispose: () => client.stop() });
}

function activate(context) {
  startLsp(context);
  context.subscriptions.push(diagnostics);

  // 保存时诊断（不打断输入；.formula 是"写完再算"的资产）
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => diagnose(doc)),
    vscode.workspace.onDidOpenTextDocument((doc) => diagnose(doc)),
    vscode.workspace.onDidCloseTextDocument((doc) => diagnostics.delete(doc.uri))
  );

  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      { language: 'himari-formula' },
      {
        async provideCompletionItems(doc) {
          const items = [];
          for (const fn of FUNCTIONS) {
            const it = new vscode.CompletionItem(fn, vscode.CompletionItemKind.Function);
            it.detail = 'himari 函数（一元）';
            items.push(it);
          }
          for (const { kind, name } of await symbols(doc)) {
            const it = new vscode.CompletionItem(
              name,
              kind === 'decl' ? vscode.CompletionItemKind.Variable : vscode.CompletionItemKind.Value
            );
            it.detail = kind === 'decl' ? '声明符号' : '本文件公式名';
            items.push(it);
          }
          return items;
        },
      },
      ' ', '\n', '=', '*', '+', '-', '/', '^', '(', '['
    )
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('himari-formula.diagnose', async () => {
      const doc = vscode.window.activeTextEditor?.document;
      if (doc) await diagnose(doc);
    })
  );
}

module.exports = { activate };