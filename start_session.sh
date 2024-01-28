
#!/bin/bash

session="custom-copilot"
# Check if the session exists
tmux has-session -t $session 2>/dev/null

if [ $? != 0 ]; then
    tmux new-session -d -s $session

    tmux send-keys 'export RUST_LOG="sqlx=error,info"' Enter
    tmux send-keys 'export TEST_LOG=enabled' Enter
    tmux new-window 
    tmux new-window 
    tmux new-window 
    tmux rename-window -t $session:0 'zsh'
    tmux rename-window -t $session:1 'lib'
    tmux rename-window -t $session:2 'lsp'
    tmux rename-window -t $session:3 'testing'
    tmux rename-window -t $session:4 'nvconfig'


    tmux send-keys -t $session:3 'cd testing' Enter
    tmux send-keys -t $session:3 'nv' Enter
    tmux send-keys -t $session:4 'cd ~/Documents/GitHub/.dotfiles/nvim' Enter
    tmux send-keys -t $session:4 'nv' Enter

    tmux send-keys -t $session:2 'cd lsp' Enter
    tmux send-keys -t $session:2 'nv' Enter
fi

tmux select-window -t $session:1
tmux attach-session -t $session
