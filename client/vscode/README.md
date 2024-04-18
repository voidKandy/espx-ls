# VSC*de ESPX LSP

## Usage

Future `todo!()`

## Development

### Setup your environment

```console
# Build & put espx-lsp binary into your $PATH
run build.sh

# Setup JS
cd client/vscode
npm install
```

Ensure you have this folder in your `~/.vscode/extensions` folder
I did this by running: 
`cp -r ../vscode ~/.vscode-oss/extensions/espx-ls`

### Debugging

In VSC\*ode, go to the `Run & Debug` sidebar (Ctrl + Shft + D) and click the `Debug LSP Extension` button. This will open a new VSC\*de instance with the lsp client installed.

To get the lsp server logs, run:

```console
tail -f $(echo "console.log(require('os').tmpdir())" | node)/lsp.log
```
