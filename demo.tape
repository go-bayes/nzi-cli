# nzi-cli demo recording
Output demo.gif

# terminal setup
Set FontSize 16
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"

# launch the app
Type "nzi-cli"
Enter
Sleep 5s

# move focus to weather panel first
Tab
Sleep 2s

# toggle expanded weather grid (e only works when weather is focused)
Type@100ms "e"
Sleep 5s

# cycle weather cities with spacebar
Type " "
Sleep 2s
Type " "
Sleep 2s

# back to compact
Type@100ms "e"
Sleep 2s

# move to time panel
Type@100ms "l"
Sleep 1s

# cycle destination cities
Type " "
Sleep 1s
Type " "
Sleep 1s
Type " "
Sleep 2s

# show help
Type "/help"
Enter
Sleep 3s

# close help
Escape
Sleep 2s

# quit
Type "q"
Sleep 1s
