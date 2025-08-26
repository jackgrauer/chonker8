# Text Selection Demo for chonker8-hot

## How to test selection:

1. Run: `./target/release/chonker8-hot real_test.pdf`

2. Basic typing:
   - Just type letters to insert text at cursor
   - Arrow keys move cursor (yellow highlight)

3. Selection (the feature you asked about):
   - Hold SHIFT + Arrow keys to start selecting
   - Cursor turns CYAN when selecting
   - Selected text has BLUE background
   - Status bar shows selection coordinates

4. Operations on selection:
   - Ctrl+C = Copy selection to clipboard
   - Ctrl+X = Cut selection (copy + delete)
   - Ctrl+V = Paste from clipboard
   - Type any character = Replace entire selection with that character
   - Esc = Cancel selection

## Visual indicators:
- Yellow cursor = Normal mode, not selecting
- Cyan cursor = Actively selecting with Shift held
- Blue background = Selected text area
- Status bar = Shows "Selecting (x1,y1) to (x2,y2)"

The selection is already working - try it!