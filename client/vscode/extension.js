// @ts-check
const { LanguageClient } = require("vscode-languageclient/node");
const tmpdir = require("os").tmpdir();

module.exports = {
  /** @param {import("vscode").ExtensionContext} context*/
  activate(context) {
    /** @type {import("vscode-languageclient/node").ServerOptions} */
    const serverOptions = {
      run: {
        command: "espx-copilot",
      },
      debug: {
        command: "espx-copilot",
        args: ["--file", `${tmpdir}/lsp.log`, "--level", "TRACE"],
      },
    };

    /** @type {import("vscode-languageclient/node").LanguageClientOptions} */
    const clientOptions = {
      documentSelector: [{ scheme: "file", language: "html" }],
    };

    const client = new LanguageClient(
      "espx-copilot",
      "Espionox Copilot",
      serverOptions,
      clientOptions,
    );

    client.start();
  },
};
