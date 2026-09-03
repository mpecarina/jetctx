# jetctx-demo — directory, git state, duration, and exit status change in one shared prompt.
#
# Render with the gate, not with bare vhs:
#   terminal-rec/scripts/render.sh docs/tapes/jetctx-demo.tape \
#     --expect "api-control-plane" --expect "status 1 with duration"
#
# Geometry is pixels. Budget 132x25.

Output docs/media/jetctx-demo.mp4
Output .terminal-rec/jetctx-demo.txt

Require bash
Require git
Require zsh

Set Shell "bash"
Set FontSize 16
Set Width 1414
Set Height 480
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms
Set Framerate 24
Set CursorBlink false
Set BorderRadius 8
Set WindowBar "Colorful"

# Setup nobody should watch. An explicit PS1 keeps the prompt identical on every machine.
Hide
Type "PS1='❯ ' && bash docs/tapes/jetctx-demo.sh setup && clear"
Enter
Sleep 800ms
Show

Type "bash docs/tapes/jetctx-demo.sh clean"
Sleep 1200ms
Enter
Wait+Screen@20s /clean branch/
Sleep 1200ms

Type "bash docs/tapes/jetctx-demo.sh dirty"
Sleep 1200ms
Enter
Wait+Screen@20s /uncommitted change/
Sleep 1200ms

Type "bash docs/tapes/jetctx-demo.sh success"
Sleep 1200ms
Enter
Wait+Screen@20s /successful command with duration/
Sleep 1200ms

Type "bash docs/tapes/jetctx-demo.sh failure"
Sleep 1200ms
Enter
Wait+Screen@20s /status 1 with duration/
Sleep 1200ms

Screenshot docs/media/jetctx-demo.png

# Let the end state hold, then stop. Trailing Sleep is dead documentation.
Sleep 800ms
