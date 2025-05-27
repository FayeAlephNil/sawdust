# Sawdust

This is a small modal text editor I wrote primarily as a proof of concept to myself that I could write such a thing,

It does not currently have (most) features one would want out a text editor. However, here is what's currently working
- Insertion/Deletion of Text in Insert Mode (entered with 'i')
- Writing to a File in Normal Mode with 'w'
- Quitting with 'q'
- Basic navigation with 'h j k l'
- Appending to a line with 'A'
- Inserting at beginning of line with 'I'
- Replacing a character with 'r'

The keybindings are largely the same as Vi, as this is my favorite brand of editor.

Planned features include
- An undo tree + undo button (with 'u')
- A basic command pallette
- A statusline, as the current version is harder to read
- Variable number of lines to display (currently displays 20 lines at a time)
