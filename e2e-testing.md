# Agent Hub E2E Testing

## Hub Basics
- [ ] **1. Hub tab is default on startup** — App opens with Hub tab active (not a standalone terminal) 
  - Notes:// YES works
- [ ] **2. Click sidebar session → opens IN the Hub** — No separate Terminal tab appears
  - Notes: opens in hub, but also opens new terminal...
- [ ] **3. Click 2nd session → auto-splits Hub** — Both terminals visible side by side
  - Notes: visable side by side but some thing like backspace dont work and typing is slow
- [ ] **4. Hub tab is clickable** — Switch to another tab (Cmd-T), click Hub tab, switches back
  - Notes: opens new tab...

## Hub Interactions
- [ ] **5. Pane focus highlights border** — Click different Hub panes → accent border on focused pane
  - Notes: Works
- [ ] **6. Right panel updates on focus** — Changes/Files/Notes update to focused pane's worktree 
  - Notes: not sure what this means??
- [ ] **7. Right-click → context menu** — Shows Split Right, Split Down, Close Pane
  - Notes: Shows but splits dont work close works but leaves an emty pane, says terminal session ended.
- [ ] **8. Split Right/Down** — Creates empty slot next to terminal
  - Notes: non functional
- [ ] **9. Close Pane** — Removes pane, layout collapses
  - Notes: doesnt remove pane only terminal goes away 

## Session Cards (Sidebar)
- [ ] **10. Active sessions show green dot** — Recently active sessions have green status dot
  - Notes: The dots arent chaning (grey when closed and then opening stays grey...)
- [ ] **11. Inactive sessions show compact rows** — Single-line with name + time
  - Notes: All sessions show compacted?
- [ ] **12. "Show N more" toggle** — Expands/collapses inactive sessions
  - Notes: Works
- [ ] **13. Right-click session → context menu** — Resume Session, Copy Session ID
  - Notes: yes works
- [ ] **14. Branch separators are thin ruled lines** — `⑂ main ──── +3139 -518` style
  - Notes: yes shows

## Advanced
- [ ] **15. Dividers are draggable** — Resize split ratio between Hub panes
  - Notes: only vertical divders show but they are slow and the slider doesnt stick to the mouse. 
- [ ] **16. Hub persists across restart** — Quit app, relaunch → layout structure restored
  - Notes: Cant test yet
- [ ] **17. Terminal content responds to resize** — Shrink/grow a pane, terminal reflows
  - Notes: Terminal doesnt stick inside the space it seems to get cut off
