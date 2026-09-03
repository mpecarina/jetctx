# jetctx-tmux-demo — keep jetctx visible across a real multi-pane tmux workspace.
#
# Render with:
#   terminal-rec/scripts/render.sh docs/tapes/jetctx-tmux-demo.tape \
#     --expect "api-control-plane" --expect "MEM" --expect "?? src/health.rs"

Output docs/media/jetctx-tmux-demo.mp4
Output .terminal-rec/jetctx-tmux-demo.txt

Require bash
Require git
Require tmux

Set Shell "bash"
Set FontSize 16
Set Width 1414
Set Height 560
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms
Set Framerate 24
Set CursorBlink false
Set BorderRadius 8
Set WindowBar "Colorful"

Hide
Type "PS1='❯ ' && bash docs/tapes/jetctx-tmux-demo.sh setup && clear"
Enter
Type "bash docs/tapes/jetctx-tmux-demo.sh attach"
Enter
Wait+Screen@20s /api-control-plane/
Sleep 500ms
Show
Sleep 1400ms

Type "git status --short --branch"
Sleep 1200ms
Enter
Wait+Screen@20s /## main/
Sleep 1200ms

Ctrl+b
Sleep 500ms
Type "%"
Sleep 1200ms

Type "touch src/health.rs"
Sleep 1200ms
Enter
Sleep 800ms

Type "git status --short"
Sleep 1200ms
Enter
Wait+Screen@20s /\?\? src\/health.rs/
Sleep 1200ms

Ctrl+b
Sleep 500ms
Type "o"
Sleep 900ms

Type "git status --short"
Sleep 1200ms
Enter
Wait+Screen@20s /\?\? src\/health.rs/
Sleep 1400ms

Screenshot docs/media/jetctx-tmux-demo.png
Sleep 800ms

Hide
Ctrl+b
Type "d"
Sleep 400ms
Type "bash docs/tapes/jetctx-tmux-demo.sh cleanup"
Enter
Sleep 400ms
