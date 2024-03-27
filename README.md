# Yet to be named AI LSP project

### Utilizes [Espionox](https://github.com/voidKandy/Espionox) under the hood

This project is very much a work in progress. As of right now, the LSP provides code actions for prompting a the assistant model. All you need to do is type
`#$` followed by a prompt and then request a code action from your IDE.

The assistant model is given context of your codebase, and uses vector embeddings to similarity search your prompts across summaries of your files.

# IDE setup

As of right now I only know how to get this working in NeoVim ¯\_(ツ)\_/¯

### Setup with LSPZero

First, manually compile espx-copilot and put it in your `PATH`. Assuming you have `lspconfig`, the below snippet should work. You can set `filetypes` to any filetypes you want. 
NOTE: Since the below code finds root dir by a directory containing `markerfile.txt`, the LSP will not attach unless this file is present in the project you want to use it in. This is for testing purposes. Soon a config file will be required

```
local lsp_config = require 'lspconfig'
local configs = require 'lspconfig.configs'


if not configs.espx_copilot then
  configs.espx_copilot = {
    default_config = {
      name = 'espx_copilot',
      autostart = true,
      cmd = { 'espx-copilot' },
      filetypes = { 'text', 'rust' },
      root_dir = function()
        return vim.fs.dirname(vim.fs.find({ 'markerfile.txt' }, { upward = true })[1])
      end
    },
  }
end

lsp_config.espx_copilot.setup {}
```

As you can see above, as of right now, `espx-copilot` requires that you have a `markerfile.txt` in your project's root in order for the LSP to know to attach.
This is just for testing purposes and will definately change in the future.

### CodeActions keymapping

For best experience, you should map opening the LSP codeactions selection whenever you want. I use telescope, but NeoVim supports this out of the box as well.

```
vim.keymap.set('n', '<Leader>ca', vim.lsp.buf.code_action, bufopts)
```

### window/showMessage

Ensure your config has a way to handle `window/showMessage` requests from an LSP. NeoVim does support this out of the box, but in a way that isn't very condusive to great UX.
I have the `notify` plugin and this snippet of code in your config should do the trick:

```
vim.lsp.handlers['window/showMessage'] = function(_, result, ctx)
  local notify = require 'notify'
  notify.setup {
    background_colour = '#000000',
    render = 'wrapped-compact',
    timeoute = 100,
  }
  notify(result.message)
end
```

---

From here you should be good to go!

If you have any questions, suggestions, or anything at all feel free to reach out to me at voidkandy@gmail.com

Thanks to thePrimagen for making his HTMX LSP Open Source so I could fork it and build it into this :D
