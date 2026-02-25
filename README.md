# lazytask

A terminal kanban board for your todo list. Inspired by [lazygit](https://github.com/jesseduffield/lazygit)—vim-style keys, minimal and fast.

**One board per project.** Run `lazytask` in any directory and it uses a local `tasks.json` there. No config, no accounts—just your tasks.

---

## Features

- **Kanban TUI** — Three columns: To do → In progress → Done
- **Vim-style navigation** — `j`/`k` and `h`/`l` to move; **Space** to select a task, then **h**/**l** to move it between columns
- **Quick add** — Press **i** to insert a new task without leaving the board
- **Per-project** — Each folder has its own `tasks.json`; run from any project and you get that project’s board
- **CLI fallback** — Add, move, and list tasks from the command line when you don’t need the TUI

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (for building)

---

## Installation

```bash
git clone https://github.com/YOUR_USERNAME/lazytask.git
cd lazytask
cargo install --path lazytask
```

This installs the `lazytask` binary to `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

---

## Quick start

Open the board in the current directory (creates or uses `tasks.json` here):

```bash
cd your-project
lazytask
```

Or explicitly:

```bash
lazytask board
```

---

## Board key bindings

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Move up/down in the current column |
| `h` / `l` or `←` / `→` | Move to previous/next column (or move selected task) |
| `Space` | Select task under cursor (press again to deselect) |
| `h` / `l` (with task selected) | Move selected task left/right one column |
| `i` | Insert mode — type a new task, **Enter** to add, **Esc** to cancel |
| `q` | Quit |

The current column has a **green** border; the task under the cursor has a **blue** background.

---

## CLI commands

When you don’t need the TUI:

| Command | Description |
|---------|-------------|
| `lazytask` | Open the board (default) |
| `lazytask add "Task title"` | Add a task to To do |
| `lazytask move <id> <state>` | Move task by id; state: `todo`, `in_progress`, `done` |
| `lazytask list` | Print all tasks |

---

## How it works

- **Current directory = board.** Whatever folder you run `lazytask` from is where it reads and writes `tasks.json`. No global config.
- **Same binary, many boards.** Install once, then `cd` into any repo and run `lazytask` to get that project’s board.

---

## Data format

Tasks are stored in `tasks.json` in the project directory:

```json
[
  {
    "id": 1,
    "title": "Fix the login bug",
    "state": "Todo"
  },
  {
    "id": 2,
    "title": "Write README",
    "state": "Done"
  }
]
```

You can edit this file by hand or use the CLI; the board will show the latest state next time you open it.

---

## License

MIT (or your choice — update as needed)
