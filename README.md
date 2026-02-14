# illuc

illuc is a desktop app for agentic AI software development workflows.
It helps you spin up and manage focused coding tasks, each in its own workspace, so an AI agent can
work in isolation without stepping on other changes. You can start a task,
hand it to codex, and keep an eye on what it’s doing from a built-in terminal
view. When you want to jump in yourself, illuc can open the task directly in
VS Code or your system terminal.

The goal is to make agentic work feel organized and repeatable: tasks are
tracked, outputs are visible, and each task lives in a clean workspace you can
review or discard. It’s built for people who want to collaborate with an AI
agent while staying in control of the codebase.

## Getting Started (Windows)

Illuc runs as a Windows desktop app, but most AI coding workflows work best when your tools run inside Linux. The recommended setup on Windows is:

1. Install WSL 2 (Ubuntu)
2. Install and authenticate Codex, Copilot, or both (inside WSL)
3. Download and install the latest Illuc binary from GitHub Releases

### 1) Install WSL 2 (Ubuntu)

Open **PowerShell as Administrator** and run:

```powershell
# Install WSL2 + Ubuntu (you will be prompted to reboot)
wsl --install -d Ubuntu

# After reboot: confirm you're on WSL 2
wsl --status
wsl -l -v

# Set Ubuntu as the default WSL distro (recommended)
wsl --set-default Ubuntu

# Launch Ubuntu (first run will ask you to create a Linux username/password)
wsl -d Ubuntu
```

Notes:
- If `wsl --install` fails, you likely need to enable virtualization in your BIOS/UEFI and ensure your Windows version is up to date.
- You can replace `Ubuntu` with another distro shown by `wsl --list --online`.

### 2) Install Codex and/or Copilot (inside WSL)

Run the following in your WSL Ubuntu shell:

```bash
# Update base packages
sudo apt-get update

# Node.js (used to install CLIs via npm; Copilot CLI requires a recent Node)
sudo apt-get install -y nodejs npm

# OpenAI Codex CLI (optional)
npm install -g @openai/codex
codex --version
codex login

# GitHub Copilot CLI (optional)
npm install -g @github/copilot
copilot --version

# Authenticate Codex (interactive). You'll be automatically prompted to sign in.
codex

# Authenticate Copilot (interactive). In the Copilot prompt, run: /login
copilot
```

What “authenticated” means:
- `codex login` must complete successfully (you should be able to run Codex in a repo without being prompted again).
- Copilot must be logged in (inside `copilot`, run `/login` once and complete the browser/device flow).

### 3) Download and install Illuc (Windows app)

Download the [latest release](https://github.com/martijn-heil/illuc/releases/latest) from GitHub.

On Windows, download the `.exe` asset for your system, and run it to start illuc.

> Note (Windows "unknown app" warning): Windows may show a Microsoft Defender SmartScreen prompt like "Windows protected your PC" / "Unknown publisher" when running a freshly downloaded `.exe`. If you downloaded the file from our official GitHub Releases page and you trust it, click **More info** and then **Run anyway**. If the file is blocked, you can also right-click the `.exe` -> **Properties** -> check **Unblock** -> **OK**, then run it again.

## Getting Started (macOS/Linux)

Download the latest binary from GitHub Releases and install it:

- https://github.com/martijn-heil/illuc/releases

Then install and authenticate Codex and/or Copilot in your shell (the exact commands depend on your OS and package manager).
