This is a brilliant concept for a power-user tool. The AI coding landscape in 2026 is heavily shifting toward autonomous CLI agents (like Claude Code, Codex CLI, Aider, and Gemini CLI). Building an orchestration layer to "race" them against each other or have them collaborate on local code is the exact kind of tool that 10x developers will want.

However, running multiple AI agents simultaneously on the same local repository introduces a massive physical constraint: **file collisions**. If Claude Code and Aider are modifying the same physical directory at the same time, they will overwrite each other's work and break your Git state.

Here is a breakdown of the architecture, tech stack, and a step-by-step plan to build this control center safely.

### The Secret Sauce: Git Worktrees

To allow multiple agents to work simultaneously on different branches without destroying your local files, your app must use **Git Worktrees**.

Unlike standard branching (where you switch branches in a single directory), a Git worktree allows you to check out multiple branches of the *same* repository into completely separate physical directories on your hard drive.

* **How the app uses it:** When you start a task, your app creates a base branch (e.g., `feature/login`). It then creates three sub-branches and three *worktrees* (temporary folders).
* Claude Code runs in `/repo-claude-worktree`.
* Codex CLI runs in `/repo-codex-worktree`.
* This isolates their file modifications entirely, allowing them to run in parallel.

---

### The Recommended Tech Stack

Since the primary target is Linux (with Windows/Mac as secondary) and we need robust process management, here is the ideal stack:

* **Framework:** **Tauri v2**.
* *Why:* Electron is too heavy, especially when you are already running multiple CPU/RAM-intensive AI agents locally. Tauri uses a lightweight Rust backend with a web frontend. Rust is unmatched for safely managing concurrent child processes and streams.


* **Frontend:** **React + TypeScript + TailwindCSS**.
* *Why:* Fast UI development with a massive ecosystem.


* **Terminal Emulator:** **Xterm.js**.
* *Why:* You need to display the live output of these CLI tools. Xterm.js is what VS Code uses; it will perfectly render the color codes and interactive prompts from Claude Code and Codex.


* **Backend OS Interaction:** **Rust (std::process & PTY libraries)**.
* *Why:* You will need Pseudo-Terminals (PTY) to wrap the CLI agents so your app can read their live stdout/stderr and inject your own prompts programmatically.



---

### The Execution Plan

#### Phase 1: The Single-Agent Wrapper (MVP)

Before orchestrating multiple agents, prove you can control *one* securely.

1. Build a simple Tauri window with a text input for the prompt and an Xterm.js canvas.
2. Write the Rust backend logic to spawn a child process (e.g., `claude -p "your prompt"`).
3. Pipe the stdout from the Rust child process to the React frontend to display in the terminal.

#### Phase 2: Worktree Automation & Parallel Execution

This is where the app becomes a "Control Center."

1. Implement Rust functions to execute Git commands: create a new branch, and spin up isolated Git worktrees in a `.agent-workspaces/` temp folder.
2. Update the UI to a grid layout.
3. Allow the user to select multiple installed CLI tools. Spawn simultaneous Rust child processes, pointing each agent to its specific worktree directory.

#### Phase 3: The Diff & Merge Dashboard

Once the agents finish their tasks, you need to see who did it best.

1. Build a "Review" screen.
2. The app runs a `git diff` comparing each agent's worktree branch against the original base branch.
3. Display a side-by-side or unified diff.
4. Add a "Merge" button that takes the winning agent's code, merges it into the main repository, and safely deletes the temporary worktrees.

#### Phase 4: Agent Collaboration Workflows

Instead of just racing them, chain them together.

1. **The Builder/Reviewer Loop:** Agent A (e.g., Claude Code) writes the feature. The app automatically captures the diff and feeds it to Agent B (e.g., Aider) with the prompt: *"Review this diff for security vulnerabilities and optimize the logic."*
2. **Specialization:** Point Agent A exclusively at the `/backend` folder and Agent B exclusively at the `/frontend` folder, passing shared API schemas between them as context.
