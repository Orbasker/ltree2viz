# Reproduce with: vhs demo/cli-demo.tape   (needs ltree2viz on PATH)
Output demo/assets/cli-demo.gif

Set Theme "Catppuccin Mocha"
Set FontSize 20
Set Width 1200
Set Height 760
Set Padding 24
Set TypingSpeed 55ms
Set PlaybackSpeed 1.0

Hide
Type "clear" Enter
Show

Sleep 600ms
Type "# Render a hierarchy straight from stdin — no database needed"
Enter
Sleep 800ms
Type "printf 'a\na.b\na.b.c\na.b.d\n' | ltree2viz -"
Sleep 500ms
Enter
Sleep 2.5s

Type "# Wrap it in a GitHub-ready mermaid block with --format md"
Enter
Sleep 800ms
Type "printf 'Fruits.Apple\nFruits.Banana\nGrains.Rice\n' | ltree2viz - --format md"
Sleep 500ms
Enter
Sleep 3s

Type "# Missing ancestors are synthesized so the tree always connects"
Enter
Sleep 800ms
Type "printf 'Fruits.Apple\nVegetables.Carrot\n' | ltree2viz - --title produce"
Sleep 500ms
Enter
Sleep 3s
