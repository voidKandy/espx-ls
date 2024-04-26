# Yet to be named AI LSP project

### Utilizes [espionox](https://github.com/voidKandy/espionox) under the hood

This project is very much a work in progress. As of right now, the LSP provides an interface for prompting the assistant model.

# How does it work?
Assuming you're using the default configuration, you can prompt your assistant by typing '#$', followed by a prompt. While hovering over your prompt, Goto Definition, and the prompt will be removed from your text buffer and the model will be prompted. 
The LSP will replace your prompt with a random symbol which can then be hovered over to see the your prompt & the model's response, you can also Goto Definition on the symbol to go to `conversation.md`, which is simply a markdown file of the current context of your assistant model.

# IDE setup
As of right now I only know how to get this working in NeoVim ¯\_(ツ)\_/¯

I'm working on a VsCode integration

### NeoVim Setup  

First, manually compile espx-ls and put it in your `PATH`. Assuming you have `lspconfig`, the below snippet should work. You can set `filetypes` to any filetypes you want.

```
local lsp_config = require 'lspconfig'
local configs = require 'lspconfig.configs'


if not configs.espx_ls then
  configs.espx_ls = {
    default_config = {
      name = 'espx_ls',
      autostart = true,
      cmd = { 'espx-ls' },
      filetypes = { 'text', 'rust' },
      root_dir = function()
        return vim.fs.dirname(vim.fs.find({ 'espx-ls.toml' }, { upward = true })[1])
      end
    },
  }
end

lsp_config.espx_ls.setup {}
```

As you can see above, as of right now, `espx-copilot` requires that you have a `espx-ls.toml` in your project's root in order for the LSP to know to attach.

### window/showMessage

Ensure your config has a way to handle `window/showMessage` requests from an LSP. NeoVim does support this out of the box, but in a way that isn't very condusive to great UX.
If you have the `notify` plugin and this snippet of code in your config should do the trick:

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
### Config File
Add a `espx-ls.toml` to the root of your project. In order to use the AI features of this LSP, include a `model` field with the `provider` value as either `OpenAi` or `Anthropic`. For example: 
```
[model]
provider="Anthropic"
api_key="your-api-key"
```
The `model` field is the only one required for use of this LSP, you can also optionally include: 
* paths - for configuring the paths of important files, such as `conversation.md`
* user_actions - for configuring the prefixes for using certain user actions, for example, use this if you want to change the '#$' prefix for prompting the assistant
* database - for adding long term memory storage powered by [SurrealDB](https://surrealdb.com/) [EXPERIMENTAL WIP]

---

From here you should be good to go!

If you have any questions, suggestions, or anything at all feel free to reach out to me at voidkandy@gmail.com

Thanks to thePrimagen for making his HTMX LSP Open Source so I could fork it and build it into this :D
