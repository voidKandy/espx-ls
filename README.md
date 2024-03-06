# Yet to be named AI LSP project

### Utilizes [Espionox](https://github.com/voidKandy/Espionox) under the hood

This project is very much a work in progress. As of right now, the LSP provides code actions for prompting a the assistant model. All you need to do is type
`#$` followed by a prompt and then request a code action from your IDE.

The assistant model is given context of your codebase, and uses vector embeddings to similarity search your prompts across summaries of your files.

# IDE setup

As of right now I only know how to get this working in NeoVim ¯\_(ツ)\_/¯

### Setup with LSPZero

First, manually compile espx-copilot and put it in your `PATH`. From there the below snippet should work. You can set `filetypes` to any filetypes you want.

```
lsp_zero.new_client({
    name = 'espx-copilot',
    autostart = 'true',
    cmd = { 'espx-copilot' },
    filetypes = { 'html', 'text', 'rust' },
    root_dir = function()
        return lsp_zero.dir.find_first({ 'markerfile.txt' })
    end
})
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
vim.lsp.handlers["window/showMessage"] = function(_, result, ctx)
    local notify = require("notify")
    notify.setup({
    background_colour = "#000000",
    render = "wrapped-compact",
    timeoute = 100,
    })
        local function keysToString(tbl)
            local keyString = ""
            for key, _ in pairs(tbl) do
                keyString = keyString .. key .. ", "
            end
            -- Remove the trailing comma and space
            keyString = keyString:gsub(", $", "")
            return keyString
        end

        notify(result.message)

end
```

---

From here you should be good to go!

If you have any questions, suggestions, or anything at all feel free to reach out to me at ezra@voidandy.space

Thanks to thePrimagen for making his HTMX LSP Open Source so I could fork it and build it into this :D
